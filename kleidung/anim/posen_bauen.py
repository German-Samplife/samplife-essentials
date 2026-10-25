"""Baut kleidung/anim/samplife.ifp, die Teilanimationen, die das Plugin in die DLL einbettet.

Aufruf (Python 3, ohne Zusatzpakete):

    python posen_bauen.py samplife.ifp quelle.ifp:anim=name [quelle.ifp:anim=name ...]

Jede Angabe nimmt die Animation `anim` aus der IFP-Datei `quelle.ifp` (Format ANP3, wie
GTA SA) und legt sie unter `name` in den Block `samplife`. Behalten werden nur die
Knochen des Oberkoerpers, dieselben wie bei CARRY/crry_prtial (Wirbelsaeule, Kopf,
Arme, Haende); Wurzel, Becken und Beine fallen weg, damit die Beine beim Gehen der
Laufanimation folgen. Eine starre Pose mit nur einem Keyframe bekommt einen zweiten,
gleichen eine Sekunde spaeter, damit die Animation nicht 0 lang ist.

Namen: hoechstens 23 Zeichen, nur a-z, 0-9 und _ (so prueft sie auch das Plugin).
"""
import re
import struct
import sys

BLOCK = "samplife"

# Knochen-IDs von CARRY/crry_prtial (anim.img/gta3.img): Spine, Spine1, Neck, Head,
# Brauen, Kiefer, beide Schluesselbeine bis Finger, Bauch, Brust
OBERKOERPER = {2, 3, 4, 5, 6, 7, 8, 21, 22, 23, 24, 25, 26, 31, 32, 33, 34, 35, 36, 201, 301, 302}

# Groesse eines Keyframes je Typ (1/2 unkomprimiert, 3/4 komprimiert; 2/4 mit Verschiebung)
FRAME = {1: 20, 2: 32, 3: 10, 4: 16}
# Zeit des zweiten Keyframes einer starren Pose: komprimiert in 1/60 s, unkomprimiert in s
DAUER_KOMPRIMIERT = 60
DAUER_SEKUNDEN = 1.0

NAME = re.compile(r"^[a-z0-9_]{1,23}$")


def cstr(b):
    return b.split(b"\0", 1)[0].decode("latin1")


def name24(s):
    b = s.encode("latin1")
    assert len(b) <= 23, s
    return b + b"\0" * (24 - len(b))


def lesen(pfad):
    data = open(pfad, "rb").read()
    if data[:4] != b"ANP3":
        sys.exit(f"{pfad}: kein ANP3 (GTA-SA-Format), sondern {data[:4]!r}")
    anzahl, = struct.unpack_from("<I", data, 32)
    pos, anims = 36, {}
    for _ in range(anzahl):
        name = cstr(data[pos:pos + 24])
        knochen, _, flags = struct.unpack_from("<III", data, pos + 24)
        pos += 36
        seqs = []
        for _ in range(knochen):
            sname = cstr(data[pos:pos + 24])
            typ, frames, bone = struct.unpack_from("<III", data, pos + 24)
            pos += 36
            roh = data[pos:pos + FRAME[typ] * frames]
            pos += FRAME[typ] * frames
            seqs.append((sname, typ, frames, bone, roh))
        anims[name.lower()] = (flags, seqs)
    return anims


def zwei_keyframes(typ, roh):
    """Erster Keyframe zweimal, der zweite eine Sekunde spaeter."""
    erstes = roh[:FRAME[typ]]
    if typ in (3, 4):  # Quaternion 4x int16, Zeit int16, bei 4 dahinter 3x int16
        zweites = erstes[:8] + struct.pack("<h", DAUER_KOMPRIMIERT) + erstes[10:]
    else:  # Quaternion 4x float, bei 2 Verschiebung 3x float, dann Zeit float
        zweites = erstes[:-4] + struct.pack("<f", DAUER_SEKUNDEN)
    return erstes + zweites


def teilanimation(name, flags, seqs):
    teile, groesse = [], 0
    for sname, typ, frames, bone, roh in seqs:
        if bone not in OBERKOERPER:
            continue
        if frames == 1:
            roh, frames = zwei_keyframes(typ, roh), 2
        teile.append(name24(sname) + struct.pack("<III", typ, frames, bone) + roh)
        groesse += len(roh)
    if not teile:
        sys.exit(f"{name}: keine Knochen des Oberkoerpers in der Animation")
    kopf = name24(name) + struct.pack("<III", len(teile), groesse, flags)
    return kopf + b"".join(teile), len(teile)


def main(argv):
    if len(argv) < 2:
        sys.exit(__doc__)
    ziel, angaben = argv[0], argv[1:]
    dateien, inhalt, namen = {}, b"", set()
    for angabe in angaben:
        m = re.match(r"^(.+\.ifp):([^=]+)=(.+)$", angabe, re.IGNORECASE)
        if not m:
            sys.exit(f"Angabe nicht verstanden: {angabe} (erwartet quelle.ifp:anim=name)")
        pfad, anim, name = m.group(1), m.group(2).lower(), m.group(3)
        if not NAME.match(name) or name in namen:
            sys.exit(f"Name ungueltig oder doppelt: {name}")
        namen.add(name)
        anims = dateien.setdefault(pfad, lesen(pfad))
        if anim not in anims:
            sys.exit(f"{pfad}: keine Animation {anim} (vorhanden: {', '.join(anims)})")
        roh, knochen = teilanimation(name, *anims[anim])
        inhalt += roh
        print(f"{name:24s} aus {pfad}:{anim}, {knochen} Knochen")
    inhalt = name24(BLOCK) + struct.pack("<I", len(angaben)) + inhalt
    datei = b"ANP3" + struct.pack("<I", len(inhalt)) + inhalt
    open(ziel, "wb").write(datei)
    print(f"{ziel}: Block {BLOCK}, {len(angaben)} Animationen, {len(datei)} Bytes")


if __name__ == "__main__":
    main(sys.argv[1:])
