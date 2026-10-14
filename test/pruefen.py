#!/usr/bin/env python3
"""Prueft die Installationslogik des Samplife Essentials Installers ohne Oberflaeche.

Legt ein nachgebautes GTA-Verzeichnis und Testpakete in einem temporaeren Ordner an, liefert sie ueber
einen lokalen Webserver aus und startet die gebaute .exe mit --still --test. Es wird nichts aus dem
Internet geladen und nichts ausserhalb des temporaeren Ordners geschrieben.

Aufruf:  python installer/test/pruefen.py      (vorher installer\\bauen.cmd ausfuehren)
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
import zipfile

HIER = os.path.dirname(os.path.abspath(__file__))
# Optional: Pfad zur .exe als erster Parameter
EXE = os.path.abspath(sys.argv[1]) if len(sys.argv) > 1 else os.path.join(os.path.dirname(HIER), 'ausgabe', 'Samplife Essentials Installer.exe')
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
    def log_message(self, *args):
        pass


def gta_anlegen(ordner, samp=b'samp-0.3.7-R3'):
    os.makedirs(ordner)
    open(os.path.join(ordner, 'gta_sa.exe'), 'wb').write(b'spiel')
    open(os.path.join(ordner, 'samp.dll'), 'wb').write(samp)
    open(os.path.join(ordner, 'vorbisFile.dll'), 'wb').write(b'alte fassung')


def starten(gta, manifest_url, *mehr):
    return subprocess.run([EXE, '--still', '--test', '--gta', gta, '--manifest', manifest_url] + list(mehr), timeout=120).returncode


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

    def manifest(name, komponenten):
        daten = {'version': 1,
                 'samp_versionen': [{'name': '0.3.7-R3', 'sha256': sha(b'samp-0.3.7-R3')},
                                    {'name': '0.3.7-R1', 'sha256': sha(b'samp-0.3.7-R1')}],
                 'komponenten': komponenten}
        with open(os.path.join(web, name), 'w', encoding='utf-8') as f:
            json.dump(daten, f)
        return basis + name

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
