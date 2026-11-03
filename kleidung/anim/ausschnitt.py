"""Schneidet eine Animation aus GTAs anim.img (oder einer IFP-Datei) auf einen Zeitbereich zu.

Aufruf (Python 3, ohne Zusatzpakete):

    python ausschnitt.py <anim.img|datei.ifp> <block.ifp> <animation> <von_s> <bis_s> <ziel.ifp>

Bei einer .ifp-Datei als Quelle wird <block.ifp> nicht gebraucht (beliebig, z. B. -). Behalten werden
alle Keyframes von <von_s> bis <bis_s> (Sekunden), die Zeiten beginnen danach bei 0. Das Ergebnis ist
eine ANP3 mit genau dieser einen Animation, die posen_bauen.py als Quelle nimmt. Nur komprimierte
Keyframes (Typ 3 und 4, wie in allen IFPs von GTA SA).

Beispiel, die Pose handschellen (KISSING/gift_give am Anfang: Haende zusammen hinter dem Ruecken):

    python ausschnitt.py "<GTA>/anim/anim.img" kissing.ifp gift_give 0 0.84 handschellen_quelle.ifp
"""
import struct
import sys

FRAME = {3: 10, 4: 16}


def img_datei(pfad, name):
    with open(pfad, "rb") as f:
        kopf = f.read(8)
        if kopf[:4] != b"VER2":
            sys.exit(f"{pfad}: kein IMG Version 2 (GTA SA)")
        anzahl, = struct.unpack_from("<I", kopf, 4)
        tabelle = f.read(32 * anzahl)
        for i in range(anzahl):
            start, groesse, groesse2 = struct.unpack_from("<IHH", tabelle, i * 32)
            eintrag = tabelle[i * 32 + 8:i * 32 + 32].split(b"\0", 1)[0].decode("latin1")
            if eintrag.lower() == name.lower():
                f.seek(start * 2048)
                return f.read((groesse or groesse2) * 2048)
    sys.exit(f"{pfad}: keine Datei {name}")


def ausschneiden(data, animation, von, bis):
    if data[:4] != b"ANP3":
        sys.exit(f"keine ANP3 (GTA-SA-Format), sondern {data[:4]!r}")
    anzahl, = struct.unpack_from("<I", data, 32)
    pos = 36
    for _ in range(anzahl):
        name = data[pos:pos + 24]
        knochen, _, flags = struct.unpack_from("<III", data, pos + 24)
        pos += 36
        teile, groesse = [], 0
        for _ in range(knochen):
            skopf = data[pos:pos + 24]
            typ, frames, bone = struct.unpack_from("<III", data, pos + 24)
            pos += 36
            if typ not in FRAME:
                sys.exit(f"Keyframe-Typ {typ} wird nicht unterstuetzt")
            roh = data[pos:pos + FRAME[typ] * frames]
            pos += FRAME[typ] * frames
            if name.split(b"\0", 1)[0].decode("latin1").lower() != animation.lower():
                continue
            behalten, erster = b"", None
            for k in range(frames):
                frame = roh[k * FRAME[typ]:(k + 1) * FRAME[typ]]
                zeit, = struct.unpack_from("<h", frame, 8)
                if not von * 60 - 0.5 <= zeit <= bis * 60 + 0.5:
                    continue
                if erster is None:
                    erster = zeit
                behalten += frame[:8] + struct.pack("<h", zeit - erster) + frame[10:]
            if not behalten:
                sys.exit(f"Knochen {bone}: kein Keyframe zwischen {von} und {bis} s")
            teile.append(skopf + struct.pack("<III", typ, len(behalten) // FRAME[typ], bone) + behalten)
            groesse += len(behalten)
        if teile:
            return name + struct.pack("<III", len(teile), groesse, flags) + b"".join(teile)
    sys.exit(f"keine Animation {animation}")


def main(argv):
    if len(argv) != 6:
        sys.exit(__doc__)
    quelle, block, animation, von, bis, ziel = argv
    if quelle.lower().endswith(".img"):
        data = img_datei(quelle, block)
    else:
        data = open(quelle, "rb").read()
    anim = ausschneiden(data, animation, float(von), float(bis))
    inhalt = b"ausschnitt".ljust(24, b"\0") + struct.pack("<I", 1) + anim
    open(ziel, "wb").write(b"ANP3" + struct.pack("<I", len(inhalt)) + inhalt)
    print(f"{ziel}: {animation} von {von} bis {bis} s")


if __name__ == "__main__":
    main(sys.argv[1:])
