#!/usr/bin/env python3
"""Prueft den Samplife Launcher ohne Oberflaeche.

Legt nachgebaute GTA-Verzeichnisse und Testpakete in einem temporaeren Ordner an, liefert sie ueber
einen lokalen Webserver aus und startet die gebaute .exe mit --still --test. Es wird nichts aus dem
Internet geladen. Ausserhalb des temporaeren Ordners schreibt nur der Spielername einen eigenen
Test-Schluessel in der Registry (HKCU\\Software\\German Samplife\\Launcher-Test), nicht den von SA-MP.

Aufruf:  python test/pruefen.py      (vorher bauen.cmd ausfuehren)
"""
import functools
import hashlib
import http.server
import io
import json
import os
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import zipfile

HIER = os.path.dirname(os.path.abspath(__file__))
# Optional: Pfad zur .exe als erster Parameter
EXE = os.path.abspath(sys.argv[1]) if len(sys.argv) > 1 else os.path.join(os.path.dirname(HIER), 'ausgabe', 'Samplife Launcher.exe')
fehler = []


def sha(daten):
    return hashlib.sha256(daten).hexdigest()


def zip_bauen(dateien):
    puffer = io.BytesIO()
    with zipfile.ZipFile(puffer, 'w', zipfile.ZIP_DEFLATED) as z:
        for name, inhalt in dateien.items():
            z.writestr(name, inhalt)
    return puffer.getvalue()


def pruefe(bedingung, text):
    print(('  ok      ' if bedingung else '  FEHLER  ') + text)
    if not bedingung:
        fehler.append(text)


class Stiller(http.server.SimpleHTTPRequestHandler):
    """Liefert Dateien aus, merkt sich jeden Abruf und nimmt Anmeldungen per POST an."""
    abrufe = []
    anmeldungen = []

    def log_message(self, *args):
        pass

    def do_GET(self):
        Stiller.abrufe.append(self.path)
        super().do_GET()

    def do_POST(self):
        laenge = int(self.headers.get('Content-Length', 0))
        Stiller.anmeldungen.append(json.loads(self.rfile.read(laenge).decode('utf-8')))
        if self.path.endswith('abgelehnt'):
            code, antwort = 403, {'ok': False, 'fehler': 'Testserver lehnt ab'}
        else:
            code, antwort = 200, {'ok': True, 'gueltig_bis': '2026-10-22T12:00:00Z'}
        daten = json.dumps(antwort).encode('utf-8')
        self.send_response(code)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(daten)))
        self.end_headers()
        self.wfile.write(daten)


def samp_exe_bauen(ordner):
    """Kleines Programm als Ersatz fuer samp.exe: schreibt seine Parameter und den Arbeitsordner in gestartet.txt."""
    quelle = os.path.join(ordner, 'samp.cs')
    with open(quelle, 'w', encoding='utf-8') as f:
        f.write('using System; using System.IO;\n'
                'class Samp { static void Main(string[] a) {\n'
                '  File.WriteAllText(Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "gestartet.txt"),\n'
                '    string.Join(" ", a) + "\\n" + Environment.CurrentDirectory); } }\n')
    csc = os.path.join(os.environ.get('WINDIR', r'C:\Windows'), 'Microsoft.NET', 'Framework', 'v4.0.30319', 'csc.exe')
    ziel = os.path.join(ordner, 'samp.exe')
    subprocess.run([csc, '/nologo', '/out:' + ziel, quelle], check=True, stdout=subprocess.DEVNULL)
    return ziel


def gestartet(gta, warten=10):
    """Inhalt von gestartet.txt, sobald die nachgebaute samp.exe gelaufen ist, sonst None."""
    datei = os.path.join(gta, 'gestartet.txt')
    ende = time.time() + warten
    while time.time() < ende:
        if os.path.exists(datei):
            time.sleep(0.2)
            return open(datei, encoding='utf-8').read()
        time.sleep(0.2)
    return None


def registry_name():
    import winreg
    try:
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, r'Software\German Samplife\Launcher-Test\SAMP') as schluessel:
            return winreg.QueryValueEx(schluessel, 'PlayerName')[0]
    except OSError:
        return None


def gta_anlegen(ordner, samp=b'samp-0.3.7-R3'):
    os.makedirs(ordner)
    open(os.path.join(ordner, 'gta_sa.exe'), 'wb').write(b'spiel')
    open(os.path.join(ordner, 'samp.dll'), 'wb').write(samp)
    open(os.path.join(ordner, 'vorbisFile.dll'), 'wb').write(b'alte fassung')


def starten(gta, manifest_url, *mehr, exe=None):
    return subprocess.run([exe or EXE, '--still', '--test', '--gta', gta, '--manifest', manifest_url] + list(mehr), timeout=120).returncode


def lesen(*teile):
    with open(os.path.join(*teile), 'rb') as f:
        return f.read()


def main():
    if not os.path.exists(EXE):
        sys.exit('Die .exe fehlt. Zuerst installer\\bauen.cmd ausfuehren.')
    arbeit = tempfile.mkdtemp(prefix='essentials-test-')
    web = os.path.join(arbeit, 'web')
    os.makedirs(web)

    lader = zip_bauen({'vorbisFile.dll': b'neuer lader', 'scripts/global.ini': b'[x]'})
    voice_r3 = zip_bauen({'paket/sampvoice.asi': b'voice r3', 'paket/sv/eins.dat': b'1', 'LIESMICH.txt': b'nicht entpacken'})
    voice_r1 = zip_bauen({'paket/sampvoice.asi': b'voice r1'})
    einzeln = b'einzelne datei'
    boese = zip_bauen({'../ausbruch.txt': b'x'})
    spiel = zip_bauen({'gta_sa.exe': b'ersetzt'})
    for name, daten in (('lader.zip', lader), ('voice-r3.zip', voice_r3), ('voice-r1.zip', voice_r1),
                        ('einzeln.bin', einzeln), ('boese.zip', boese), ('spiel.zip', spiel)):
        open(os.path.join(web, name), 'wb').write(daten)

    server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), functools.partial(Stiller, directory=web))
    threading.Thread(target=server.serve_forever, daemon=True).start()
    basis = 'http://127.0.0.1:%d/' % server.server_address[1]

    def manifest(name, komponenten, **mehr):
        daten = {'version': 1,
                 'samp_versionen': [{'name': '0.3.7-R3', 'sha256': sha(b'samp-0.3.7-R3')},
                                    {'name': '0.3.7-R1', 'sha256': sha(b'samp-0.3.7-R1')}],
                 'komponenten': komponenten}
        daten.update(mehr)
        with open(os.path.join(web, name), 'w', encoding='utf-8') as f:
            json.dump(daten, f)
        return basis + name

    def launcher_manifest(name, komponenten, **mehr):
        """Liste wie auf dem Server: nur R1, mit Stand und Server."""
        daten = {'version': 1, 'stand': 3, 'bilder': [basis + 'bild.jpg'], 'updates': basis + 'updates.json',
                 'samp_versionen': [{'name': '0.3.7-R1', 'sha256': sha(b'samp-0.3.7-R1')}],
                 'server': {'adresse': 'german-samplife.test', 'port': 7777, 'anmeldung': basis + 'api/launcher/start'},
                 'komponenten': komponenten}
        daten.update(mehr)
        with open(os.path.join(web, name), 'w', encoding='utf-8') as f:
            json.dump(daten, f)
        return basis + name

    def downloads():
        return [a for a in Stiller.abrufe if a.endswith('.zip') or a.endswith('.bin')]

    def paket(datei, daten, **mehr):
        eintrag = {'url': basis + datei, 'sha256': sha(daten), 'groesse': len(daten)}
        eintrag.update(mehr)
        return eintrag

    gut = [
        {'id': 'lader', 'name': 'ASI Loader', 'pflicht': True, 'pakete': [paket('lader.zip', lader)]},
        {'id': 'voice', 'name': 'SampVoice', 'pakete': [
            paket('voice-r1.zip', voice_r1, samp='0.3.7-R1', zip_unterordner='paket'),
            paket('voice-r3.zip', voice_r3, samp='0.3.7-R3', zip_unterordner='paket')]},
        {'id': 'einzeln', 'name': 'Einzeldatei', 'pakete': [
            paket('einzeln.bin', einzeln, typ='datei', ziel='cef', dateiname='cef-client.dll')]},
    ]

    try:
        print('1. Normale Installation (SA-MP 0.3.7-R3)')
        gta = os.path.join(arbeit, 'gta1')
        gta_anlegen(gta)
        pruefe(starten(gta, manifest('gut.json', gut)) == 0, 'Rueckgabe 0')
        pruefe(lesen(gta, 'vorbisFile.dll') == b'neuer lader', 'vorhandene Datei ersetzt')
        sicherungen = os.path.join(gta, 'samplife-essentials-sicherung')
        gesichert = [os.path.join(w, n) for w, _, ns in os.walk(sicherungen) for n in ns] if os.path.isdir(sicherungen) else []
        pruefe(len(gesichert) == 1 and lesen(gesichert[0]) == b'alte fassung', 'alte Fassung gesichert')
        pruefe(lesen(gta, 'scripts', 'global.ini') == b'[x]', 'Unterordner aus dem Archiv angelegt')
        pruefe(lesen(gta, 'sampvoice.asi') == b'voice r3', 'Paket der passenden SA-MP-Version gewaehlt')
        pruefe(os.path.exists(os.path.join(gta, 'sv', 'eins.dat')), 'zip_unterordner: Inhalt ohne den Ordner entpackt')
        pruefe(not os.path.exists(os.path.join(gta, 'LIESMICH.txt')), 'zip_unterordner: Rest des Archivs ausgelassen')
        pruefe(lesen(gta, 'cef', 'cef-client.dll') == b'einzelne datei', 'Einzeldatei mit Zielordner und Dateiname')
        pruefe(lesen(gta, 'gta_sa.exe') == b'spiel' and lesen(gta, 'samp.dll') == b'samp-0.3.7-R3', 'Spiel und SA-MP unveraendert')
        protokoll = lesen(gta, 'samplife-essentials-protokoll.txt').decode('utf-8')
        pruefe(basis + 'voice-r3.zip' in protokoll, 'Protokoll nennt die Quelle des Downloads')

        print('2. Zweiter Lauf: nichts mehr zu tun')
        pruefe(starten(gta, basis + 'gut.json') == 0, 'Rueckgabe 0')
        gesichert2 = [n for _, _, ns in os.walk(sicherungen) for n in ns]
        pruefe(len(gesichert2) == 1, 'keine weitere Sicherung, Dateien waren aktuell')

        print('3. Andere SA-MP-Version (0.3.7-R1)')
        gta = os.path.join(arbeit, 'gta2')
        gta_anlegen(gta, b'samp-0.3.7-R1')
        pruefe(starten(gta, basis + 'gut.json') == 0, 'Rueckgabe 0')
        pruefe(lesen(gta, 'sampvoice.asi') == b'voice r1', 'Paket fuer R1 gewaehlt')

        print('4. Falsche Pruefsumme')
        gta = os.path.join(arbeit, 'gta3')
        gta_anlegen(gta)
        falsch = [{'id': 'lader', 'name': 'ASI Loader', 'pakete': [dict(paket('lader.zip', lader), sha256='0' * 64)]}]
        pruefe(starten(gta, manifest('falsch.json', falsch)) == 1, 'Rueckgabe 1')
        pruefe(lesen(gta, 'vorbisFile.dll') == b'alte fassung', 'nichts geschrieben')

        print('5. Archiv will aus dem GTA-Verzeichnis ausbrechen')
        gta = os.path.join(arbeit, 'gta4')
        gta_anlegen(gta)
        pruefe(starten(gta, manifest('boese.json', [{'id': 'b', 'name': 'Boese', 'pakete': [paket('boese.zip', boese)]}])) == 1, 'Rueckgabe 1')
        pruefe(not os.path.exists(os.path.join(arbeit, 'ausbruch.txt')), 'keine Datei ausserhalb angelegt')

        print('6. Archiv will gta_sa.exe ersetzen')
        pruefe(starten(gta, manifest('spiel.json', [{'id': 's', 'name': 'Spiel', 'pakete': [paket('spiel.zip', spiel)]}])) == 1, 'Rueckgabe 1')
        pruefe(lesen(gta, 'gta_sa.exe') == b'spiel', 'gta_sa.exe unveraendert')

        print('7. Kein GTA-Verzeichnis')
        leer = os.path.join(arbeit, 'leer')
        os.makedirs(leer)
        pruefe(starten(leer, basis + 'gut.json') == 2, 'Rueckgabe 2')

        print('8. Unverschluesselte Adresse ausserhalb des eigenen Rechners')
        gta = os.path.join(arbeit, 'gta5')
        gta_anlegen(gta)
        fremd = [{'id': 'f', 'name': 'Fremd', 'pakete': [{'url': 'http://example.com/x.zip', 'sha256': '0' * 64, 'groesse': 1}]}]
        pruefe(starten(gta, manifest('fremd.json', fremd)) == 1, 'Rueckgabe 1, kein Download ueber http')

        print('9. Liste nicht erreichbar, Ersatzadresse antwortet')
        gta = os.path.join(arbeit, 'gta6')
        gta_anlegen(gta)
        pruefe(starten(gta, basis + 'gibt-es-nicht.json', '--ausweich', basis + 'gut.json') == 0, 'Rueckgabe 0 ueber die Ersatzadresse')
        pruefe(lesen(gta, 'sampvoice.asi') == b'voice r3', 'Pakete aus der Ersatzliste installiert')

        print('10. Liste nicht erreichbar, keine Ersatzadresse')
        gta = os.path.join(arbeit, 'gta7')
        gta_anlegen(gta)
        pruefe(starten(gta, basis + 'gibt-es-nicht.json') == 3, 'Rueckgabe 3, nichts installiert')
        pruefe(not os.path.exists(os.path.join(gta, 'sampvoice.asi')), 'keine Datei geschrieben')

        werkzeug = os.path.join(arbeit, 'werkzeug')
        os.makedirs(werkzeug)
        samp_exe = samp_exe_bauen(werkzeug)

        def spiel_anlegen(ordner, samp=b'samp-0.3.7-R1', mit_exe=True):
            gta_anlegen(ordner, samp)
            if mit_exe:
                shutil.copy(samp_exe, os.path.join(ordner, 'samp.exe'))
            return ordner

        def spielen(gta, url, *mehr, **schluessel):
            gestartet_datei = os.path.join(gta, 'gestartet.txt')
            if os.path.exists(gestartet_datei):
                os.remove(gestartet_datei)
            return starten(gta, url, '--spielen', '--name', 'Max_Mustermann', *mehr, **schluessel)

        lader2 = zip_bauen({'vorbisFile.dll': b'noch neuerer lader', 'scripts/global.ini': b'[x]'})
        extra = b'optionaler mod'
        for name, daten in (('lader2.zip', lader2), ('extra.bin', extra)):
            open(os.path.join(web, name), 'wb').write(daten)
        spiel_liste = [
            {'id': 'lader', 'name': 'ASI Loader', 'pakete': [paket('lader.zip', lader)]},
            {'id': 'voice', 'name': 'SampVoice', 'pakete': [paket('voice-r1.zip', voice_r1, samp='0.3.7-R1', zip_unterordner='paket')]},
        ]
        url = launcher_manifest('spiel.json', spiel_liste)

        print('11. Spielen: laden, Stand sichern, anmelden, samp.exe starten')
        gta = spiel_anlegen(os.path.join(arbeit, 'spiel1'))
        Stiller.anmeldungen.clear()
        pruefe(spielen(gta, url) == 0, 'Rueckgabe 0')
        pruefe(os.path.exists(os.path.join(gta, 'samplife-launcher', 'stand.json')), 'stand.json angelegt')
        pruefe(lesen(gta, 'samplife-launcher', 'manifest.json') == lesen(web, 'spiel.json'), 'Manifest Byte fuer Byte gesichert')
        stand = json.loads(lesen(gta, 'samplife-launcher', 'stand.json').decode('utf-8'))
        dateien = [d['pfad'] for k in stand['komponenten'] for p in k['pakete'] for d in p['dateien']]
        pruefe(stand['stand'] == 3 and 'sampvoice.asi' in dateien and 'vorbisFile.dll' in dateien, 'Stand nennt Stand 3 und alle Dateien')
        anmeldung = Stiller.anmeldungen[-1] if Stiller.anmeldungen else {}
        pruefe(anmeldung.get('name') == 'Max_Mustermann' and anmeldung.get('stand') == 3 and anmeldung.get('launcher', '').startswith('2.'),
               'Anmeldung mit Name, Stand und Launcher-Version')
        start = gestartet(gta)
        pruefe(start is not None and start.split('\n')[0].strip() == 'german-samplife.test:7777', 'samp.exe mit Adresse und Port gestartet')
        # Windows nennt den Temp-Ordner mal kurz (RUNNER~1), mal lang; deshalb ueber samefile vergleichen
        pruefe(start is not None and os.path.samefile(start.split('\n')[1].strip(), gta), 'Arbeitsordner ist das GTA-Verzeichnis')
        pruefe(registry_name() == 'Max_Mustermann', 'Name fuer SA-MP gesetzt (Test-Schluessel)')

        print('12. Zweiter Start: nichts zu laden')
        vorher = len(downloads())
        pruefe(spielen(gta, url) == 0, 'Rueckgabe 0')
        pruefe(len(downloads()) == vorher, 'kein Download')
        pruefe(gestartet(gta) is not None, 'samp.exe gestartet')

        print('13. Veraltetes Paket')
        url2 = launcher_manifest('spiel2.json', [dict(spiel_liste[0], pakete=[paket('lader2.zip', lader2)]), spiel_liste[1]], stand=4)
        vorher = len(downloads())
        pruefe(spielen(gta, url2) == 0, 'Rueckgabe 0')
        pruefe(downloads()[vorher:] == ['/lader2.zip'], 'nur das veraltete Paket geladen')
        pruefe(lesen(gta, 'vorbisFile.dll') == b'noch neuerer lader', 'neue Fassung abgelegt')

        print('14. Veraenderte Datei wird repariert')
        open(os.path.join(gta, 'sampvoice.asi'), 'wb').write(b'VOICE R1')
        vorher = len(downloads())
        pruefe(spielen(gta, url2) == 0, 'Rueckgabe 0')
        pruefe(downloads()[vorher:] == ['/voice-r1.zip'], 'nur das betroffene Paket neu geladen')
        pruefe(lesen(gta, 'sampvoice.asi') == b'voice r1', 'Datei wiederhergestellt')

        print('15. Gruendlich: gleiche Groesse, gleiche Zeit, anderer Inhalt')
        asi = os.path.join(gta, 'sampvoice.asi')
        zeit = os.stat(asi).st_mtime_ns
        open(asi, 'wb').write(b'voice R1')
        os.utime(asi, ns=(zeit, zeit))
        vorher = len(downloads())
        pruefe(starten(gta, url2) == 0 and len(downloads()) == vorher, 'schneller Abgleich laesst es durch (erwartet)')
        pruefe(starten(gta, url2, '--gruendlich') == 0 and downloads()[vorher:] == ['/voice-r1.zip'], 'gruendlich: neu geladen')
        pruefe(lesen(gta, 'sampvoice.asi') == b'voice r1', 'Datei wiederhergestellt')

        print('16. Server-Liste nicht erreichbar: Start mit der Sicherung')
        vorher = len(downloads())
        Stiller.anmeldungen.clear()
        pruefe(spielen(gta, basis + 'weg.json') == 0, 'Rueckgabe 0')
        pruefe(len(downloads()) == vorher, 'nichts geladen')
        pruefe(gestartet(gta) is not None, 'samp.exe gestartet')
        pruefe(len(Stiller.anmeldungen) == 1 and Stiller.anmeldungen[0].get('stand') == 4, 'Anmeldung mit dem gesicherten Stand')

        print('17. Ohne Server-Liste und mit fehlender Datei')
        os.remove(os.path.join(gta, 'sampvoice.asi'))
        pruefe(spielen(gta, basis + 'weg.json') == 3, 'Rueckgabe 3')
        pruefe(gestartet(gta, 2) is None, 'kein Start')

        print('18. Ohne Server-Liste, Sicherung beschaedigt')
        gta = spiel_anlegen(os.path.join(arbeit, 'spiel2'))
        pruefe(spielen(gta, url) == 0, 'erst einmal mit Liste')
        with open(os.path.join(gta, 'samplife-launcher', 'manifest.json'), 'ab') as f:
            f.write(b' ')
        pruefe(spielen(gta, basis + 'weg.json') == 3, 'Rueckgabe 3, Pruefsumme der Sicherung stimmt nicht')
        pruefe(spielen(gta, url) == 0, 'mit Liste wieder in Ordnung')
        # Noch ein Lauf: Danach ist auch die vorige Fassung (.alt) wieder stimmig
        pruefe(spielen(gta, url) == 0, 'zweiter Lauf mit Liste')
        with open(os.path.join(gta, 'samplife-launcher', 'manifest.json'), 'ab') as f:
            f.write(b' ')
        pruefe(spielen(gta, basis + 'weg.json') == 0, 'mit der vorigen Sicherung (.alt) weiter')

        print('19. Optionale Komponente')
        url3 = launcher_manifest('optional.json', spiel_liste, optionale_komponenten=[
            {'id': 'extra', 'name': 'Extra', 'pakete': [paket('extra.bin', extra, typ='datei', dateiname='extra.asi')]}])
        gta = spiel_anlegen(os.path.join(arbeit, 'spiel3'))
        pruefe(spielen(gta, url3) == 0 and not os.path.exists(os.path.join(gta, 'extra.asi')), 'ohne Wahl nicht geladen')
        pruefe(spielen(gta, url3, '--optional', 'extra') == 0 and lesen(gta, 'extra.asi') == extra, 'mit Wahl geladen')
        pruefe(spielen(gta, url3) == 0 and os.path.exists(os.path.join(gta, 'extra.asi')), 'abgewaehlt: Datei bleibt liegen')

        print('20. Voraussetzungen fuer den Start')
        gta = spiel_anlegen(os.path.join(arbeit, 'spiel4'))
        pruefe(starten(gta, url, '--spielen', '--name', 'Max Mustermann') == 5, 'Name mit Leerzeichen: Rueckgabe 5')
        gta = spiel_anlegen(os.path.join(arbeit, 'spiel5'), samp=b'samp-0.3.7-R3')
        pruefe(spielen(gta, url) == 4, 'SA-MP R3 statt R1: Rueckgabe 4')
        gta = spiel_anlegen(os.path.join(arbeit, 'spiel6'), mit_exe=False)
        pruefe(spielen(gta, url) == 4, 'ohne samp.exe: Rueckgabe 4')
        pruefe(not os.path.exists(os.path.join(gta, 'sampvoice.asi')), 'nichts geladen, bevor der Start moeglich ist')

        print('21. Anmeldung abgelehnt: Spiel startet trotzdem')
        url4 = launcher_manifest('abgelehnt.json', spiel_liste,
                                 server={'adresse': 'german-samplife.test', 'port': 7777, 'anmeldung': basis + 'api/launcher/abgelehnt'})
        gta = spiel_anlegen(os.path.join(arbeit, 'spiel7'))
        pruefe(spielen(gta, url4) == 0, 'Rueckgabe 0')
        pruefe(gestartet(gta) is not None, 'samp.exe gestartet')
        pruefe(len(Stiller.anmeldungen) > 0, 'Anmeldung versucht')

        print('22. Selbstaktualisierung')
        kopie = os.path.join(arbeit, 'kopie')
        os.makedirs(kopie)
        exe_kopie = os.path.join(kopie, 'Samplife Launcher.exe')
        shutil.copy(EXE, exe_kopie)
        neu = b'neue fassung des launchers'
        open(os.path.join(web, 'launcher-neu.bin'), 'wb').write(neu)
        gta = spiel_anlegen(os.path.join(arbeit, 'spiel8'))
        falsch = launcher_manifest('launcher-falsch.json', spiel_liste, launcher={
            'version': '9.0.0', 'url': basis + 'launcher-neu.bin', 'sha256': '0' * 64, 'groesse': len(neu)})
        pruefe(starten(gta, falsch, '--kein-neustart', exe=exe_kopie) == 7, 'falsche Pruefsumme: Rueckgabe 7')
        pruefe(lesen(exe_kopie) == lesen(EXE), 'laufende Datei unveraendert')
        richtig = launcher_manifest('launcher-richtig.json', spiel_liste, launcher={
            'version': '9.0.0', 'url': basis + 'launcher-neu.bin', 'sha256': sha(neu), 'groesse': len(neu)})
        pruefe(starten(gta, richtig, '--kein-neustart', exe=exe_kopie) == 10, 'Rueckgabe 10')
        pruefe(lesen(exe_kopie) == neu, 'neue Fassung an der Stelle der alten')
        pruefe(lesen(exe_kopie + '.alt') == lesen(EXE), 'alte Fassung als .alt')
        gleich = launcher_manifest('launcher-gleich.json', spiel_liste, launcher={
            'version': '2.0.0', 'url': basis + 'launcher-neu.bin', 'sha256': sha(neu), 'groesse': len(neu)})
        pruefe(starten(gta, gleich) == 0, 'gleiche Version: keine Aktualisierung')

        print('23. Adresse samplife:// fuer den Knopf „Verbinden“')
        pruefe(subprocess.run([EXE, '--adresse-eintragen', '--test'], timeout=60).returncode == 0, 'Rueckgabe 0')
        import winreg
        try:
            with winreg.OpenKey(winreg.HKEY_CURRENT_USER, r'Software\Classes\samplife-test\shell\open\command') as schluessel:
                befehl = winreg.QueryValueEx(schluessel, '')[0]
            with winreg.OpenKey(winreg.HKEY_CURRENT_USER, r'Software\Classes\samplife-test') as schluessel:
                winreg.QueryValueEx(schluessel, 'URL Protocol')
                protokoll = True
        except OSError:
            befehl, protokoll = '', False
        pruefe(protokoll, 'als URL-Protokoll eingetragen (Test-Schema)')
        pruefe(os.path.normcase(EXE) in os.path.normcase(befehl) and '--link "%1"' in befehl, 'Befehl zeigt auf die .exe mit --link')

        print('24. SA-MP mitliefern, wenn es fehlt')
        samp_paket = zip_bauen({'samp.exe': lesen(samp_exe), 'samp.dll': b'samp-0.3.7-R1', 'samp.saa': b'saa',
                                'SAMP/SAMP.img': b'img'})
        boeses_samp = zip_bauen({'samp.dll': b'fremd'})
        for name, daten in (('samp-client.zip', samp_paket), ('samp-boese.zip', boeses_samp)):
            open(os.path.join(web, name), 'wb').write(daten)
        url5 = launcher_manifest('mit-samp.json', spiel_liste, samp_client={
            'id': 'samp', 'name': 'SA-MP 0.3.7-R1', 'pakete': [paket('samp-client.zip', samp_paket)]})

        def ohne_samp(ordner):
            os.makedirs(ordner)
            open(os.path.join(ordner, 'gta_sa.exe'), 'wb').write(b'spiel')
            return ordner

        gta = ohne_samp(os.path.join(arbeit, 'ohne-samp'))
        pruefe(spielen(gta, url5) == 0, 'Rueckgabe 0')
        pruefe(lesen(gta, 'samp.dll') == b'samp-0.3.7-R1' and os.path.exists(os.path.join(gta, 'SAMP', 'SAMP.img')), 'SA-MP eingerichtet')
        pruefe(gestartet(gta) is not None, 'samp.exe aus dem Paket gestartet')
        stand = json.loads(lesen(gta, 'samplife-launcher', 'stand.json').decode('utf-8'))
        pruefe('samp' not in [k['id'] for k in stand['komponenten']], 'SA-MP steht nicht im Stand (wird nie wieder ersetzt)')
        import winreg
        try:
            with winreg.OpenKey(winreg.HKEY_CURRENT_USER, r'Software\German Samplife\Launcher-Test\SAMP') as schluessel:
                gta_eintrag = winreg.QueryValueEx(schluessel, 'gta_sa_exe')[0]
        except OSError:
            gta_eintrag = ''
        pruefe(os.path.normcase(gta_eintrag) == os.path.normcase(os.path.join(gta, 'gta_sa.exe')), 'gta_sa_exe fuer SA-MP eingetragen')
        vorher = len([a for a in Stiller.abrufe if a.endswith('samp-client.zip')])
        pruefe(spielen(gta, url5) == 0, 'zweiter Start: Rueckgabe 0')
        pruefe(len([a for a in Stiller.abrufe if a.endswith('samp-client.zip')]) == vorher, 'SA-MP nicht noch einmal geladen')

        print('25. Vorhandenes SA-MP wird nie ersetzt')
        open(os.path.join(gta, 'samp.dll'), 'wb').write(b'samp-0.3.7-R3')
        pruefe(spielen(gta, url5) == 4, 'andere Version: Rueckgabe 4')
        pruefe(lesen(gta, 'samp.dll') == b'samp-0.3.7-R3', 'samp.dll unveraendert')
        gta = ohne_samp(os.path.join(arbeit, 'ohne-samp2'))
        boese_liste = launcher_manifest('samp-boese.json', [{'id': 'b', 'name': 'Boese', 'pakete': [paket('samp-boese.zip', boeses_samp)]}])
        pruefe(starten(gta, boese_liste) == 1, 'andere Komponente darf samp.dll nicht ablegen: Rueckgabe 1')
        pruefe(not os.path.exists(os.path.join(gta, 'samp.dll')), 'keine samp.dll abgelegt')
    finally:
        server.shutdown()
        shutil.rmtree(arbeit, ignore_errors=True)

    print()
    if fehler:
        print('%d Pruefung(en) fehlgeschlagen.' % len(fehler))
        sys.exit(1)
    print('Alle Pruefungen bestanden.')


if __name__ == '__main__':
    main()
