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
        daten = json.loads(self.rfile.read(laenge).decode('utf-8'))
        Stiller.anmeldungen.append(daten)
        if self.path.endswith('abgelehnt'):
            code, antwort = 403, {'ok': False, 'fehler': 'Testserver lehnt ab'}
        elif 'passwort' in daten and daten['passwort'] != 'richtig':
            code, antwort = 401, {'ok': False, 'fehler': 'Name oder Passwort stimmt nicht.'}
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


def gta_exe_bauen(ordner):
    """Ersatz fuer gta_sa.exe (32 Bit wie das Spiel): schreibt seine Parameter, getrennt durch |, in gta-gestartet.txt und wartet bis zu
    zehn Sekunden darauf, dass eine samp.dll in den Prozess geladen wird; deren Pfad steht dann in gta-geladen.txt."""
    quelle = os.path.join(ordner, 'gta.cs')
    with open(quelle, 'w', encoding='utf-8') as f:
        f.write('using System; using System.Diagnostics; using System.IO; using System.Threading;\n'
                'class Gta { static void Main(string[] a) {\n'
                '  var hier = AppDomain.CurrentDomain.BaseDirectory;\n'
                '  File.WriteAllText(Path.Combine(hier, "gta-gestartet.txt"), string.Join("|", a));\n'
                '  for (var i = 0; i < 100; i++) {\n'
                '    foreach (ProcessModule m in Process.GetCurrentProcess().Modules)\n'
                '      if (m.ModuleName.Equals("samp.dll", StringComparison.OrdinalIgnoreCase)) {\n'
                '        File.WriteAllText(Path.Combine(hier, "gta-geladen.txt"), m.FileName); return; }\n'
                '    Thread.Sleep(100); } } }\n')
    csc = os.path.join(os.environ.get('WINDIR', r'C:\Windows'), 'Microsoft.NET', 'Framework', 'v4.0.30319', 'csc.exe')
    ziel = os.path.join(ordner, 'gta_sa.exe')
    subprocess.run([csc, '/nologo', '/platform:x86', '/out:' + ziel, quelle], check=True, stdout=subprocess.DEVNULL)
    return ziel


def gleiche_datei(a, b):
    """Windows nennt Pfade mal kurz (RUNNER~1), mal lang; deshalb ueber samefile vergleichen."""
    try:
        gleich = a is not None and os.path.samefile(a.strip(), b)
    except OSError:
        gleich = False
    if not gleich:
        print('          erwartet ' + b + ', erhalten ' + repr(a))
    return gleich


def warten_auf(datei, sekunden=15):
    ende = time.time() + sekunden
    while time.time() < ende:
        if os.path.exists(datei):
            time.sleep(0.2)
            return open(datei, encoding='utf-8').read()
        time.sleep(0.2)
    return None


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
        os.remove(os.path.join(gta, 'samp.dll'))
        pruefe(spielen(gta, url, '--appdaten', os.path.join(arbeit, 'appdaten-leer')) == 4, 'ohne SA-MP und ohne samp_client: Rueckgabe 4')
        pruefe(not os.path.exists(os.path.join(gta, 'sampvoice.asi')), 'nichts geladen, bevor der Start moeglich ist')

        print('21. Anmeldung abgelehnt: Spiel startet trotzdem')
        url4 = launcher_manifest('abgelehnt.json', spiel_liste,
                                 server={'adresse': 'german-samplife.test', 'port': 7777, 'anmeldung': basis + 'api/launcher/abgelehnt'})
        gta = spiel_anlegen(os.path.join(arbeit, 'spiel7'))
        pruefe(spielen(gta, url4) == 0, 'Rueckgabe 0')
        pruefe(gestartet(gta) is not None, 'samp.exe gestartet')
        pruefe(len(Stiller.anmeldungen) > 0, 'Anmeldung versucht')

        print('21b. Passwort des Accounts')
        gta = spiel_anlegen(os.path.join(arbeit, 'konto'))
        pruefe(spielen(gta, url, '--kontopasswort', 'falsch') == 8, 'falsches Passwort: Rueckgabe 8')
        pruefe(gestartet(gta, 2) is None, 'kein Start')
        pruefe(Stiller.anmeldungen[-1].get('passwort') == 'falsch', 'Passwort an die Anmeldung gegeben')
        pruefe(spielen(gta, url, '--kontopasswort', 'richtig') == 0, 'richtiges Passwort: Rueckgabe 0')
        pruefe(gestartet(gta) is not None, 'samp.exe gestartet')
        pruefe(spielen(gta, url) == 0 and 'passwort' not in Stiller.anmeldungen[-1], 'ohne Passwort: Feld fehlt in der Anmeldung')

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
        pruefe(spielen(gta, url5, '--appdaten', os.path.join(arbeit, 'appdaten-24')) == 0, 'Rueckgabe 0')
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
        pruefe(spielen(gta, url5, '--appdaten', os.path.join(arbeit, 'appdaten-24')) == 0, 'zweiter Start: Rueckgabe 0')
        pruefe(len([a for a in Stiller.abrufe if a.endswith('samp-client.zip')]) == vorher, 'SA-MP nicht noch einmal geladen')

        print('25. Vorhandenes SA-MP wird nie ersetzt')
        open(os.path.join(gta, 'samp.dll'), 'wb').write(b'samp-0.3.7-R3')
        pruefe(spielen(gta, url5, '--appdaten', os.path.join(arbeit, 'appdaten-24')) == 4, 'andere Version: Rueckgabe 4')
        pruefe(lesen(gta, 'samp.dll') == b'samp-0.3.7-R3', 'samp.dll unveraendert')
        gta = ohne_samp(os.path.join(arbeit, 'ohne-samp2'))
        boese_liste = launcher_manifest('samp-boese.json', [{'id': 'b', 'name': 'Boese', 'pakete': [paket('samp-boese.zip', boeses_samp)]}])
        pruefe(starten(gta, boese_liste) == 1, 'andere Komponente darf samp.dll nicht ablegen: Rueckgabe 1')
        pruefe(not os.path.exists(os.path.join(gta, 'samp.dll')), 'keine samp.dll abgelegt')

        # Start wie beim open.mp-Launcher: gta_sa.exe starten und samp.dll laden. Als samp.dll dient eine Kopie der
        # 32-Bit-version.dll von Windows; die Liste nennt deren Pruefsumme als 0.3.7-R1.
        gta_exe = gta_exe_bauen(werkzeug)
        dll_daten = lesen(os.environ.get('WINDIR', r'C:\Windows'), 'SysWOW64', 'version.dll')
        open(os.path.join(web, 'samp-r1.dll'), 'wb').write(dll_daten)
        geteilt = zip_bauen({'gta/SAMP/SAMP.img': b'img', 'gta/samp.saa': b'saa vom paket', 'gta/sampgui.png': b'png'})
        open(os.path.join(web, 'samp-geteilt.zip'), 'wb').write(geteilt)

        def lade_manifest(name, **mehr):
            daten = {'version': 1, 'stand': 3,
                     'samp_versionen': [{'name': '0.3.7-R1', 'sha256': sha(dll_daten)}],
                     'server': {'adresse': '127.0.0.1', 'port': 7777, 'anmeldung': basis + 'api/launcher/start'},
                     'komponenten': spiel_liste}
            daten.update(mehr)
            with open(os.path.join(web, name), 'w', encoding='utf-8') as f:
                json.dump(daten, f)
            return basis + name

        def nur_gta(ordner):
            os.makedirs(ordner)
            shutil.copy(gta_exe, os.path.join(ordner, 'gta_sa.exe'))
            open(os.path.join(ordner, 'vorbisFile.dll'), 'wb').write(b'alte fassung')
            return ordner

        print('26. samp.dll des open.mp-Launchers laden')
        appdaten = os.path.join(arbeit, 'appdaten-openmp')
        os.makedirs(os.path.join(appdaten, 'mp.open.launcher', 'samp', '0.3.7-R1'))
        openmp_dll = os.path.join(appdaten, 'mp.open.launcher', 'samp', '0.3.7-R1', 'samp.dll')
        open(openmp_dll, 'wb').write(dll_daten)
        gta = nur_gta(os.path.join(arbeit, 'laden1'))
        pruefe(spielen(gta, lade_manifest('laden1.json'), '--appdaten', appdaten) == 0, 'Rueckgabe 0')
        parameter = warten_auf(os.path.join(gta, 'gta-gestartet.txt'))
        pruefe(parameter == '-c|-n|Max_Mustermann|-h|127.0.0.1|-p|7777', 'gta_sa.exe mit den Parametern von SA-MP gestartet')
        geladen = warten_auf(os.path.join(gta, 'gta-geladen.txt'))
        pruefe(gleiche_datei(geladen, openmp_dll), 'samp.dll des open.mp-Launchers geladen')
        pruefe(not os.path.exists(os.path.join(gta, 'samp.dll')), 'keine samp.dll im GTA-Verzeichnis abgelegt')

        print('26b. Mit Serverpasswort')
        os.remove(os.path.join(gta, 'gta-gestartet.txt'))
        os.remove(os.path.join(gta, 'gta-geladen.txt'))
        pruefe(spielen(gta, lade_manifest('laden1.json'), '--appdaten', appdaten, '--passwort', 'geheim wort') == 0, 'Rueckgabe 0')
        pruefe(warten_auf(os.path.join(gta, 'gta-gestartet.txt')) == '-c|-n|Max_Mustermann|-h|127.0.0.1|-p|7777|-z|geheim wort',
               'Passwort mit -z an gta_sa.exe, als ein Parameter')
        gta = nur_gta(os.path.join(arbeit, 'laden-exe'))
        shutil.copy(samp_exe, os.path.join(gta, 'samp.exe'))
        open(os.path.join(gta, 'samp.dll'), 'wb').write(dll_daten)
        pruefe(spielen(gta, lade_manifest('laden1.json'), '--appdaten', appdaten, '--passwort', 'geheim') == 0, 'samp.exe vorhanden: Rueckgabe 0')
        pruefe(gleiche_datei(warten_auf(os.path.join(gta, 'gta-geladen.txt')), os.path.join(gta, 'samp.dll')),
               'mit Passwort samp.dll aus dem GTA-Verzeichnis geladen statt samp.exe')
        pruefe(not os.path.exists(os.path.join(gta, 'gestartet.txt')), 'samp.exe nicht gestartet')

        print('27. samp_client: samp.dll in den Launcher-Ordner, vorhandene Dateien bleiben')
        appdaten = os.path.join(arbeit, 'appdaten-eigen')
        url6 = lade_manifest('laden2.json', samp_client={'id': 'samp', 'name': 'SA-MP 0.3.7-R1', 'pakete': [
            paket('samp-geteilt.zip', geteilt, zip_unterordner='gta'),
            paket('samp-r1.dll', dll_daten, typ='datei', ablage='launcher', ziel='samp/0.3.7-R1', dateiname='samp.dll')]})
        gta = nur_gta(os.path.join(arbeit, 'laden2'))
        open(os.path.join(gta, 'samp.saa'), 'wb').write(b'eigene saa')
        pruefe(spielen(gta, url6, '--appdaten', appdaten) == 0, 'Rueckgabe 0')
        eigene_dll = os.path.join(appdaten, 'German Samplife', 'Launcher', 'samp', '0.3.7-R1', 'samp.dll')
        pruefe(os.path.exists(eigene_dll) and lesen(eigene_dll) == dll_daten, 'samp.dll im Launcher-Ordner')
        pruefe(lesen(gta, 'samp.saa') == b'eigene saa', 'vorhandene samp.saa unveraendert')
        pruefe(lesen(gta, 'SAMP', 'SAMP.img') == b'img', 'fehlende Dateien ins GTA-Verzeichnis gelegt')
        pruefe(not os.path.exists(os.path.join(gta, 'samp.dll')), 'keine samp.dll im GTA-Verzeichnis')
        geladen = warten_auf(os.path.join(gta, 'gta-geladen.txt'))
        pruefe(gleiche_datei(geladen, eigene_dll), 'eigene samp.dll geladen')
        vorher = len([a for a in Stiller.abrufe if a.endswith('samp-r1.dll')])
        os.remove(os.path.join(gta, 'gta-geladen.txt'))
        pruefe(spielen(gta, url6, '--appdaten', appdaten) == 0, 'zweiter Start: Rueckgabe 0')
        pruefe(len([a for a in Stiller.abrufe if a.endswith('samp-r1.dll')]) == vorher, 'SA-MP nicht noch einmal geladen')
        pruefe(warten_auf(os.path.join(gta, 'gta-geladen.txt')) is not None, 'wieder geladen')

        print('28. Liste: Ablage nur fuer samp_client')
        gta = nur_gta(os.path.join(arbeit, 'laden3'))
        url7 = lade_manifest('laden3.json', komponenten=[{'id': 'x', 'name': 'X', 'pakete': [
            paket('samp-r1.dll', dll_daten, typ='datei', ablage='launcher', ziel='samp/0.3.7-R1', dateiname='samp.dll')]}])
        pruefe(starten(gta, url7, '--appdaten', os.path.join(arbeit, 'appdaten-x')) == 3, 'Liste abgelehnt: Rueckgabe 3')

        def deinstallieren(gta, appdaten):
            return subprocess.run([EXE, '--still', '--test', '--deinstallieren', '--gta', gta, '--appdaten', appdaten],
                                  timeout=120).returncode

        def gibt_es(*teile):
            return os.path.exists(os.path.join(*teile))

        print('29. Deinstallieren nach der Installation')
        ad = os.path.join(arbeit, 'appdaten-weg')
        url8 = launcher_manifest('entfernen.json', [spiel_liste[0], dict(spiel_liste[1], entfernen=['voice.log', 'voice-cache'])])
        gta = spiel_anlegen(os.path.join(arbeit, 'weg1'))
        open(os.path.join(gta, 'eigener-mod.asi'), 'wb').write(b'mod')
        pruefe(spielen(gta, url8, '--appdaten', ad) == 0, 'installiert')
        verz = json.loads(lesen(gta, 'samplife-launcher', 'installiert.json').decode('utf-8'))
        eintraege = {d['pfad']: d for d in verz['dateien']}
        pruefe(eintraege.get('vorbisFile.dll', {}).get('vorher') == 'gesichert' and eintraege.get('sampvoice.asi', {}).get('vorher') == 'neu'
               and not verz['geschaetzt'], 'installiert.json: vorbisFile.dll gesichert, sampvoice.asi neu')
        open(os.path.join(gta, 'voice.log'), 'w').write('log')
        os.makedirs(os.path.join(gta, 'voice-cache', 'tief'))
        open(os.path.join(gta, 'voice-cache', 'tief', 'x'), 'w').write('x')
        os.makedirs(os.path.join(ad, 'German Samplife', 'Launcher', 'bilder'), exist_ok=True)
        open(os.path.join(ad, 'German Samplife', 'Launcher', 'einstellungen.json'), 'w').write('{}')
        pruefe(deinstallieren(gta, ad) == 0, 'Rueckgabe 0')
        pruefe(lesen(gta, 'vorbisFile.dll') == b'alte fassung', 'ersetzte Datei in der Fassung von vorher zurueck')
        pruefe(not gibt_es(gta, 'sampvoice.asi') and not gibt_es(gta, 'scripts'), 'Dateien und dadurch leere Ordner entfernt')
        pruefe(not gibt_es(gta, 'voice.log') and not gibt_es(gta, 'voice-cache'), 'im Spiel angelegte Dateien (entfernen) entfernt')
        pruefe(not any(gibt_es(gta, n) for n in ('samplife-launcher', 'samplife-essentials-sicherung', 'samplife-essentials-protokoll.txt')),
               'Stand, Sicherungen und Protokoll entfernt')
        pruefe(lesen(gta, 'eigener-mod.asi') == b'mod' and lesen(gta, 'gta_sa.exe') == b'spiel' and lesen(gta, 'samp.dll') == b'samp-0.3.7-R1',
               'eigene Mods, Spiel und SA-MP bleiben')
        pruefe(not gibt_es(ad, 'German Samplife'), 'Ordner des Launchers unter AppData entfernt')

        print('30. Unveraendert vorgefundene Datei bleibt')
        gta = spiel_anlegen(os.path.join(arbeit, 'weg2'))
        open(os.path.join(gta, 'sampvoice.asi'), 'wb').write(b'voice r1')
        pruefe(spielen(gta, url8, '--appdaten', ad) == 0 and deinstallieren(gta, ad) == 0, 'installiert und deinstalliert')
        pruefe(lesen(gta, 'sampvoice.asi') == b'voice r1', 'sampvoice.asi lag schon so da und bleibt')

        print('31. Nach einer Aktualisierung: Fassung von vor dem Launcher, nicht die alte eigene')
        gta = spiel_anlegen(os.path.join(arbeit, 'weg3'))
        pruefe(spielen(gta, url, '--appdaten', ad) == 0, 'erste Fassung')
        time.sleep(1.1)
        pruefe(spielen(gta, url2, '--appdaten', ad) == 0 and lesen(gta, 'vorbisFile.dll') == b'noch neuerer lader', 'aktualisiert')
        pruefe(deinstallieren(gta, ad) == 0 and lesen(gta, 'vorbisFile.dll') == b'alte fassung', 'Original zurueck')

        print('32. Installation eines Launchers bis 2.0 (ohne installiert.json)')
        for name, ohne_protokoll in (('weg4', False), ('weg5', True)):
            gta = spiel_anlegen(os.path.join(arbeit, name))
            spielen(gta, url, '--appdaten', ad)
            time.sleep(1.1)
            spielen(gta, url2, '--appdaten', ad)
            os.remove(os.path.join(gta, 'samplife-launcher', 'installiert.json'))
            if ohne_protokoll:
                os.remove(os.path.join(gta, 'samplife-essentials-protokoll.txt'))
            art = 'ohne Protokoll, aus der aeltesten Sicherung' if ohne_protokoll else 'aus dem Protokoll'
            pruefe(deinstallieren(gta, ad) == 0, art + ': Rueckgabe 0')
            pruefe(lesen(gta, 'vorbisFile.dll') == b'alte fassung' and not gibt_es(gta, 'sampvoice.asi'), art + ': Original zurueck, Rest entfernt')

        print('33. Fremde ASI Loader und Konflikte')
        url9 = launcher_manifest('konflikte.json', spiel_liste, konflikte=[{'datei': 'alt.asi', 'grund': 'Alte Fassung'}, {'datei': '../weg.asi'}])
        gta = spiel_anlegen(os.path.join(arbeit, 'fremd1'))
        open(os.path.join(gta, 'dinput8.dll'), 'wb').write(b'MZ\x00lade *.asi\x00 aus scripts')
        open(os.path.join(gta, 'd3d9.dll'), 'wb').write(b'MZ\x00kein loader, nur asin()')
        open(os.path.join(gta, 'alt.asi'), 'wb').write(b'alt')
        aus = os.path.join(arbeit, 'fremde.txt')

        def fremde(*mehr):
            if os.path.exists(aus):
                os.remove(aus)
            code = starten(gta, url9, '--fremde', aus, *mehr)
            return code, open(aus, encoding='utf-8-sig').read().split() if os.path.exists(aus) else []

        code, zeilen = fremde()
        pruefe(code == 0 and 'dinput8.dll|Loader' in zeilen and 'alt.asi|Konflikt' in zeilen, 'Loader und Konflikt gefunden')
        pruefe(not any(z.startswith('d3d9.dll') or z.startswith('vorbisFile.dll') for z in zeilen), 'keine Fehlalarme (d3d9.dll, Original-vorbisFile.dll)')
        open(os.path.join(gta, 'vorbisFile.dll'), 'wb').write(b'fremder lader *.asi')
        code, zeilen = fremde()
        pruefe('vorbisFile.dll|LoaderErsetzt' in zeilen, 'fremder Loader als vorbisFile.dll erkannt')
        code, zeilen = fremde('--abschalten')
        pruefe(not gibt_es(gta, 'dinput8.dll') and lesen(gta, 'dinput8.dll.samplife-aus').startswith(b'MZ')
               and gibt_es(gta, 'alt.asi.samplife-aus') and gibt_es(gta, 'vorbisFile.dll'), 'ausgeschaltet (umbenannt), vorbisFile.dll bleibt')
        pruefe(spielen(gta, url9, '--appdaten', ad) == 0, 'danach installiert')
        code, zeilen = fremde()
        pruefe(zeilen == [], 'nach der Installation nichts Fremdes mehr')
        pruefe(deinstallieren(gta, ad) == 0, 'deinstalliert')
        pruefe(gibt_es(gta, 'dinput8.dll') and gibt_es(gta, 'alt.asi') and not gibt_es(gta, 'dinput8.dll.samplife-aus'), 'wieder eingeschaltet')
        pruefe(lesen(gta, 'vorbisFile.dll') == b'fremder lader *.asi', 'eigener Loader des Spielers zurueck')

        print('34. DirectX-Laufzeit')
        quelle = os.path.join(werkzeug, 'dx.cs')
        with open(quelle, 'w', encoding='utf-8') as f:
            f.write('using System; using System.IO; using System.Reflection;\n'
                    'class Dx { static int Main(string[] a) {\n'
                    '  var ich = Assembly.GetExecutingAssembly().Location;\n'
                    '  foreach (var x in a) if (x.StartsWith("/T:")) { var ziel = x.Substring(3).Trim(\'"\');\n'
                    '    Directory.CreateDirectory(ziel); File.Copy(ich, Path.Combine(ziel, "DXSETUP.exe"), true); return 0; }\n'
                    '  foreach (var x in a) if (x == "/silent") { var sys = Environment.GetEnvironmentVariable("SAMPLIFE_TEST_SYSTEM");\n'
                    '    File.WriteAllText(Path.Combine(sys, "d3dx9_25.dll"), "x"); File.WriteAllText(Path.Combine(sys, "d3dx9_43.dll"), "x"); return 0; }\n'
                    '  return 5; } }\n')
        csc = os.path.join(os.environ.get('WINDIR', r'C:\Windows'), 'Microsoft.NET', 'Framework', 'v4.0.30319', 'csc.exe')
        dx_exe = os.path.join(werkzeug, 'dxredist.exe')
        subprocess.run([csc, '/nologo', '/out:' + dx_exe, quelle], check=True, stdout=subprocess.DEVNULL)
        dx_daten = lesen(dx_exe)
        open(os.path.join(web, 'dx.exe'), 'wb').write(dx_daten)
        system = os.path.join(arbeit, 'system')
        os.makedirs(system)
        os.environ['SAMPLIFE_TEST_SYSTEM'] = system

        def dx_eintrag(**mehr):
            eintrag = dict({'id': 'directx', 'name': 'DirectX-Laufzeit', 'typ': 'directx', 'dateien': ['d3dx9_25.dll', 'd3dx9_43.dll']},
                           **paket('dx.exe', dx_daten))
            eintrag.update(mehr)
            return eintrag

        unbekannt = {'id': 'zukunft', 'name': 'Unbekannt', 'typ': 'gibt-es-noch-nicht', 'url': 'https://example.com/x', 'sha256': '0' * 64}
        url10 = launcher_manifest('directx.json', spiel_liste, systemvoraussetzungen=[dx_eintrag(), unbekannt])
        dx_abrufe = lambda: len([a for a in Stiller.abrufe if a.endswith('/dx.exe')])
        gta = spiel_anlegen(os.path.join(arbeit, 'dx1'))
        pruefe(spielen(gta, url10, '--appdaten', ad, '--systemordner', system) == 0, 'Rueckgabe 0, unbekannter Typ uebergangen')
        pruefe(gibt_es(system, 'd3dx9_25.dll') and gibt_es(system, 'd3dx9_43.dll'), 'DirectX eingerichtet')
        pruefe(gestartet(gta) is not None, 'samp.exe gestartet')
        vorher = dx_abrufe()
        pruefe(spielen(gta, url10, '--appdaten', ad, '--systemordner', system) == 0 and dx_abrufe() == vorher, 'vorhanden: nicht noch einmal geladen')
        pruefe(deinstallieren(gta, ad) == 0 and gibt_es(system, 'd3dx9_25.dll'), 'Deinstallieren laesst DirectX da')
        leer_system = os.path.join(arbeit, 'system-leer')
        os.makedirs(leer_system)
        url11 = launcher_manifest('directx-falsch.json', spiel_liste, systemvoraussetzungen=[dx_eintrag(sha256='0' * 64)])
        gta = spiel_anlegen(os.path.join(arbeit, 'dx2'))
        pruefe(spielen(gta, url11, '--appdaten', ad, '--systemordner', leer_system) == 1, 'falsche Pruefsumme: Rueckgabe 1')
        pruefe(gestartet(gta, 2) is None and not gibt_es(leer_system, 'd3dx9_25.dll'), 'kein Start, nichts eingerichtet')
        gta = spiel_anlegen(os.path.join(arbeit, 'dx3'))
        for name in ('d3dx9_25.dll', 'd3dx9_43.dll'):
            open(os.path.join(gta, name), 'wb').write(b'x')
        vorher = dx_abrufe()
        pruefe(spielen(gta, url11, '--appdaten', ad, '--systemordner', leer_system) == 0 and dx_abrufe() == vorher,
               'DLLs im GTA-Verzeichnis genuegen')

        print('35. Fenster startet und laeuft (Selbsttest mit dem echten Fenster)')
        # 2.1.0 stuerzte mit gemerktem Verzeichnis beim Start ab, ohne dass ein Test es merkte
        for name, gemerkt, mit_loader in (('start1', True, False), ('start2', True, True), ('start3', False, False)):
            gta = spiel_anlegen(os.path.join(arbeit, name))
            if mit_loader:
                open(os.path.join(gta, 'dinput8.dll'), 'wb').write(b'MZ\x00lade *.asi\x00')
            app = os.path.join(arbeit, 'appdaten-' + name)
            os.makedirs(os.path.join(app, 'German Samplife', 'Launcher'))
            if gemerkt:
                with open(os.path.join(app, 'German Samplife', 'Launcher', 'einstellungen.json'), 'w', encoding='utf-8') as f:
                    json.dump({'gta': gta, 'optional': []}, f)
            code = subprocess.run([EXE, '--test', '--selbsttest', '--appdaten', app, '--manifest', url9], timeout=120).returncode
            art = ('mit gemerktem Verzeichnis' if gemerkt else 'ohne gemerktes Verzeichnis') + (' und fremdem Loader' if mit_loader else '')
            pruefe(code == 0, art + ': laeuft und beendet sich sauber (Rueckgabe %d)' % code)

        print('36. Neues Release auf GitHub, auch ohne Eintrag in der Liste')
        gta = spiel_anlegen(os.path.join(arbeit, 'spiel-release'))

        def release(name, tag='v9.0.0', datei='Samplife.Launcher.exe', pruefsumme='sha256:' + sha(neu), summen=False, **mehr):
            """Antwort wie von api.github.com/repos/.../releases/latest."""
            eintrag = {'name': datei, 'size': len(neu), 'browser_download_url': basis + 'launcher-neu.bin'}
            if pruefsumme:
                eintrag['digest'] = pruefsumme
            dateien = [eintrag]
            if summen:
                with open(os.path.join(web, 'summen.txt'), 'w', encoding='ascii') as f:
                    f.write('%s  Samplife.Launcher.exe\n%s  Samplife.Essentials.Installer.exe\n' % (sha(neu), sha(neu)))
                dateien.append({'name': 'SHA256SUMS.txt', 'size': 1, 'browser_download_url': basis + 'summen.txt'})
            daten = {'tag_name': tag, 'draft': False, 'prerelease': False, 'body': 'Text', 'assets': dateien}
            daten.update(mehr)
            with open(os.path.join(web, name), 'w', encoding='utf-8') as f:
                json.dump(daten, f)
            return basis + name

        def frische_kopie():
            shutil.copy(EXE, exe_kopie)
            for rest in (exe_kopie + '.alt', exe_kopie + '.neu'):
                if os.path.exists(rest):
                    os.remove(rest)

        def mit_release(liste, url, *mehr):
            frische_kopie()
            return starten(gta, liste, '--kein-neustart', '--release', url, *mehr, exe=exe_kopie)

        ohne_launcher = launcher_manifest('ohne-launcher.json', spiel_liste)
        pruefe(mit_release(ohne_launcher, release('release.json')) == 10, 'neueres Release: Rueckgabe 10')
        pruefe(lesen(exe_kopie) == neu, 'neue Fassung aus dem Release an der Stelle der alten')
        pruefe(lesen(exe_kopie + '.alt') == lesen(EXE), 'alte Fassung als .alt')
        pruefe(mit_release(ohne_launcher, release('release-summen.json', pruefsumme='', summen=True)) == 10
               and lesen(exe_kopie) == neu, 'ohne digest: Pruefsumme aus SHA256SUMS.txt')
        pruefe(mit_release(basis + 'liste-fehlt.json', release('release.json')) == 10 and lesen(exe_kopie) == neu,
               'Liste nicht erreichbar: aktualisiert trotzdem')
        # Nicht passend oder nicht neuer: keine Aktualisierung, der Start laeuft normal weiter
        for name, text, mehr in (
                ('release-falsch.json', 'falsche Pruefsumme', {'pruefsumme': 'sha256:' + '0' * 64}),
                ('release-ohne-summe.json', 'ohne Pruefsumme', {'pruefsumme': ''}),
                ('release-kleidung.json', 'Tag eines anderen Pakets', {'tag': 'kleidung-9.0.0'}),
                ('release-vorab.json', 'Vorab-Release', {'prerelease': True}),
                ('release-entwurf.json', 'Entwurf', {'draft': True}),
                ('release-andere-datei.json', 'ohne Samplife.Launcher.exe', {'datei': 'Samplife.Essentials.Installer.exe'}),
                ('release-alt.json', 'aeltere Version', {'tag': 'v1.0.0'})):
            code = mit_release(ohne_launcher, release(name, **mehr))
            pruefe(code == 0 and lesen(exe_kopie) == lesen(EXE), text + ': weiter mit dieser Fassung (Rueckgabe %d)' % code)
        code = mit_release(ohne_launcher, basis + 'release-fehlt.json')
        pruefe(code == 0 and lesen(exe_kopie) == lesen(EXE), 'GitHub antwortet nicht: weiter mit dieser Fassung (Rueckgabe %d)' % code)
        code = mit_release(ohne_launcher, release('release.json'), '--nach-aktualisierung', '9.0.0')
        pruefe(code == 0 and lesen(exe_kopie) == lesen(EXE), 'direkt nach einer Aktualisierung: keine zweite (Rueckgabe %d)' % code)
        # Was die Liste verlangt, bleibt Pflicht, auch wenn das Release aelter ist
        code = mit_release(falsch, release('release-acht.json', tag='v8.0.0'))
        pruefe(code == 7 and lesen(exe_kopie) == lesen(EXE), 'Liste verlangt 9.0.0 mit falscher Pruefsumme: Rueckgabe 7 (%d)' % code)
        pruefe(mit_release(falsch, release('release-zehn.json', tag='v10.0.0')) == 10 and lesen(exe_kopie) == neu,
               'Release neuer als die Liste: das Release gewinnt')
        # Mit Fenster: klappt die freiwillige Aktualisierung nicht, laeuft der Launcher normal weiter
        frische_kopie()
        app = os.path.join(arbeit, 'appdaten-release')
        os.makedirs(app)
        vorher = len(Stiller.abrufe)
        code = subprocess.run([exe_kopie, '--test', '--selbsttest', '--appdaten', app, '--manifest', url9,
                               '--release', release('release-fenster.json', pruefsumme='sha256:' + '0' * 64)], timeout=120).returncode
        abrufe = Stiller.abrufe[vorher:]
        pruefe(code == 0 and lesen(exe_kopie) == lesen(EXE), 'Fenster: falsche Pruefsumme, laeuft weiter (Rueckgabe %d)' % code)
        pruefe('/launcher-neu.bin' in abrufe, 'Fenster: neue Fassung versucht zu laden')
        pruefe(abrufe.count('/' + url9.rsplit('/', 1)[1]) >= 2, 'Fenster: Liste danach erneut geladen')
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
