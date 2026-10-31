// Filterscript zum Testen des Kleidungs-Plugins mit dem Server (siehe kleidung/README.md).
// Braucht die samp-cef-Server-Komponente (jede ab f14fe4f) und deren cef.inc. Beim
// Spieler: samp-cef-Client ab 0x000300 (cef/plugin-ereignisse.patch) und
// samplife-kleidung.dll in <GTA>/cef/plugins/.
//
// Laden per RCON: /rcon loadfs kleidung_test, entladen: /rcon unloadfs kleidung_test.
// Beim Laden fragt es alle verbundenen Clients (kleidung:frage); wer das Plugin hat,
// meldet sich mit kleidung:hallo. Beim Entladen vergessen alle Clients die Outfits.
//
// Spieler ohne Skin 0 sehen nichts davon; vorher z. B. /kleidung skin. Der Skin gilt
// dann nur bis zum nächsten Spawn; dauerhaft setzt ihn ein Gamemode-Befehl
// (bei German Samplife /setskin [ID] 0).
//
// Befehle:
//   /kleidung skin   eigenen Skin auf 0 (CJ) setzen
//   /kleidung 1      Outfit 1 (schwarzes Unterhemd, Afro, Tarnhose, Cowboystiefel, Kette, Brille)
//   /kleidung 2      Outfit 2 (roter Anzug, Glatze, Anzughose, schwarze Schuhe, Uhr, Cowboyhut)
//   /kleidung 3      Outfit 3 (Hoodie, Jogginghose, Turnschuhe, Cap, Tattoo am Arm)
//   /kleidung 4      Outfit 4 mit Farben (gelbes T-Shirt, gelbe Hose, rotes Kopftuch), ab Plugin 0.9.2
//   /kleidung 5      Outfit 5 mit Farben (lila Kapuzenjacke, schwarze Hose, rote Cap), ab Plugin 0.9.2
//   /kleidung aus    eigenes Outfit vergessen, alle sehen den Standard-CJ
//   /kleidung info   wer das Plugin gemeldet hat und wer welches Outfit trägt
//   /kleidung frage  alle Clients fragen, ob das Plugin läuft (Antwort kleidung:hallo)
//   /farbe <id> <stelle> <textur/modell> <RRGGBB>
//                    setzt beim Spieler <id> an Stelle <stelle> dieses Teil in dieser Farbe, z. B.
//                    /farbe 0 0 tshirtwhite/tshirt BE00FF. Farben nur an Stelle 0 (Oberteil) und 2 (Hose), das Plugin
//                    färbt nichts anderes. Der Rest des Outfits bleibt.
//                    Hatte der Spieler noch kein Outfit, beginnt es beim Standard-CJ.
//   /farbe <id> <stelle> <textur/modell> aus
//                    dasselbe Teil ohne Farbe (an jeder Stelle außer den Tattoos 4 bis 12)
//
// Farben (@RRGGBB hinter einem Teil) schickt dieses Script an Plugins ab 0.9.2, gefärbt werden nur Oberteil und Hose. Clients mit älterem Plugin bekommen dasselbe
// Outfit ohne Farben, so wie es der Gamemode auch macht.

#define FILTERSCRIPT

#include <open.mp>
#include <cef>

// 18 Teile in der Reihenfolge von eClothesTexturePart: Oberkörper, Kopf, Beine,
// Schuhe, neun Tattoo-Stellen, Kette, Uhr, Brille, Hut, Spezial. Namen wie in der
// Kleidungsliste von MTA (CClientPlayerClothes.cpp); Tattoos beginnen mit ihrer Stelle
// (4weed an Stelle 4). textur/modell, bei Tattoos nur textur, "-" für nichts.
// Chatzeilen ohne Umlaute: Diese Datei ist UTF-8, SA-MP erwartet Windows-1252.
// Ab Plugin 0.9.0 darf hinter jedem Teil außer den Tattoos eine Farbe stehen: textur/modell@RRGGBB.
static const OUTFIT_TEILE[5][] =
{
	"vestblack/vest afro/afro worktrcamogrn/worktr cowboyboot/biker - - - - - - - - - neckgold/neck2 - glasses04dark/glasses04 - -",
	"suit1red/suit1 bald/head suit1trred/suit1tr shoedressblk/shoe - - - - - - - - - - watchgno/watch - cowboy/cowboy -",
	"hoodyabase5/hoodya player_face/head tracktrgang/tracktr sneakerbincblk/sneaker 4weed - - - - - - - - - - - capblk/cap -",
	"tshirtwhite/tshirt@FFDC00 player_face/head chinosbiege/chinosb@FFDC00 sneakerprowht/sneaker - - - - - - - - - - - - bandred/bandana -",
	"hoodjackbeige/hoodjack@8000C0 player_face/head worktrgrey/worktr@000000 sneakerbincblk/sneaker - - - - - - - - - - - - capred/cap -"
};

// Standard-CJ des Plugins, Ausgang für /farbe, wenn jemand noch kein Outfit hat
static const OUTFIT_STANDARD[] = "vest/vest player_face/head jeansdenim/jeans sneakerbincblk/sneaker - - - - - - - - - - - - - -";

#define TEILE_ANZAHL 18
#define TEIL_LAENGE 40
// reicht für 18 Teile mit Farbe (ein langes Teil wie capblueover/capovereye@FFDC00 hat 29 Zeichen)
#define OUTFIT_LAENGE 400

// gOutfit: Nummer aus /kleidung (1 bis 5), 0 = mit /farbe geändert, -1 = keins
static gOutfit[MAX_PLAYERS] = {-1, ...};
static gOutfitText[MAX_PLAYERS][OUTFIT_LAENGE];
static gPluginVersion[MAX_PLAYERS][16];

public OnFilterScriptInit()
{
	cef_subscribe("kleidung:hallo", "KleidungHallo");

	// Die Clients haben sich schon beim Verbinden gemeldet, bevor das Filterscript lief
	for (new i = 0; i < MAX_PLAYERS; i++)
	{
		if (IsPlayerConnected(i))
		{
			cef_emit_event(i, "kleidung:frage");
		}
	}
	print("[Kleidung] Test-Filterscript geladen, alle Clients gefragt");
	return 1;
}

public OnFilterScriptExit()
{
	for (new i = 0; i < MAX_PLAYERS; i++)
	{
		if (IsPlayerConnected(i) && gOutfit[i] != -1)
		{
			KleidungAnAlleWeg(i);
		}
	}
	return 1;
}

public OnPlayerConnect(playerid)
{
	gOutfit[playerid] = -1;
	gPluginVersion[playerid][0] = '\0';
	return 1;
}

public OnPlayerDisconnect(playerid, reason)
{
	gOutfit[playerid] = -1;
	gPluginVersion[playerid][0] = '\0';
	KleidungAnAlleWeg(playerid);
	return 1;
}

// Antwort des Plugins: nach dem Verbinden mit dem CEF-Server und auf kleidung:frage.
// Erst jetzt kann der Client Outfits annehmen; er bekommt die aller Spieler.
forward KleidungHallo(player_id, const arguments[]);
public KleidungHallo(player_id, const arguments[])
{
	strmid(gPluginVersion[player_id], arguments, 0, strlen(arguments), sizeof(gPluginVersion[]));

	new text[96];
	format(text, sizeof(text), "[Kleidung] Plugin %s gemeldet, Outfits werden gesendet.", arguments);
	SendClientMessage(player_id, 0x007DFFFF, text);
	printf("[Kleidung] Spieler %d: kleidung:hallo %s", player_id, arguments);

	for (new i = 0; i < MAX_PLAYERS; i++)
	{
		if (IsPlayerConnected(i) && gOutfit[i] != -1)
		{
			KleidungSenden(player_id, i);
		}
	}
	return 1;
}

// Farben schickt der Server ab Plugin 0.9.2 (gemeldet in kleidung:hallo); 0.9.0 und 0.9.1 waren nur Testbauten
static bool:PluginKannFarben(spieler)
{
	new punkt = strfind(gPluginVersion[spieler], ".");
	if (punkt == -1)
	{
		return false;
	}
	new major = strval(gPluginVersion[spieler]), minor = strval(gPluginVersion[spieler][punkt + 1]), patch = 0;
	new punkt2 = strfind(gPluginVersion[spieler], ".", false, punkt + 1);
	if (punkt2 != -1)
	{
		patch = strval(gPluginVersion[spieler][punkt2 + 1]);
	}
	return major > 0 || minor > 9 || (minor == 9 && patch >= 2);
}

// Outfit ohne "@RRGGBB" für ältere Plugins, die ein Teil mit @ sonst samt Outfit verwerfen
static OhneFarben(const teile[], ziel[], groesse)
{
	new j = 0;
	for (new i = 0; teile[i] != '\0' && j < groesse - 1; i++)
	{
		if (teile[i] == '@')
		{
			while (teile[i + 1] != '\0' && teile[i + 1] != ' ')
			{
				i++;
			}
			continue;
		}
		ziel[j++] = teile[i];
	}
	ziel[j] = '\0';
}

static KleidungSenden(empfaenger, spieler)
{
	if (PluginKannFarben(empfaenger))
	{
		cef_emit_event(empfaenger, "kleidung:spieler", CEFINT(spieler), CEFSTR(gOutfitText[spieler]));
	}
	else
	{
		new ohne[OUTFIT_LAENGE];
		OhneFarben(gOutfitText[spieler], ohne, sizeof(ohne));
		cef_emit_event(empfaenger, "kleidung:spieler", CEFINT(spieler), CEFSTR(ohne));
	}
}

// Nächstes Wort ab `index` (durch Leerzeichen getrennt) nach `ziel`; false, wenn keins mehr kommt
static bool:NaechstesWort(const text[], &index, ziel[], groesse)
{
	while (text[index] == ' ')
	{
		index++;
	}
	if (text[index] == '\0')
	{
		ziel[0] = '\0';
		return false;
	}
	new j = 0;
	while (text[index] != '\0' && text[index] != ' ')
	{
		if (j < groesse - 1)
		{
			ziel[j++] = text[index];
		}
		index++;
	}
	ziel[j] = '\0';
	return true;
}

// Ersetzt im Outfit von `spieler` das Teil an Stelle `stelle`, alle anderen bleiben
static bool:TeilSetzen(spieler, stelle, const teil[])
{
	new neu[OUTFIT_LAENGE], wort[TEIL_LAENGE], index = 0;
	for (new i = 0; i < TEILE_ANZAHL; i++)
	{
		if (!NaechstesWort(gOutfitText[spieler], index, wort, sizeof(wort)))
		{
			return false;
		}
		if (i > 0)
		{
			strcat(neu, " ");
		}
		if (i == stelle)
		{
			strcat(neu, teil);
		}
		else
		{
			strcat(neu, wort);
		}
	}
	strmid(gOutfitText[spieler], neu, 0, strlen(neu), OUTFIT_LAENGE);
	return true;
}

static bool:IstHex(const text[])
{
	if (strlen(text) != 6)
	{
		return false;
	}
	for (new i = 0; i < 6; i++)
	{
		new c = text[i];
		if (!(('0' <= c <= '9') || ('a' <= c <= 'f') || ('A' <= c <= 'F')))
		{
			return false;
		}
	}
	return true;
}

static bool:IstName(const text[])
{
	new laenge = strlen(text);
	if (laenge == 0 || laenge > 24)
	{
		return false;
	}
	for (new i = 0; i < laenge; i++)
	{
		new c = text[i];
		if (!(('0' <= c <= '9') || ('a' <= c <= 'z') || ('A' <= c <= 'Z') || c == '_'))
		{
			return false;
		}
	}
	return true;
}

// /farbe <id> <stelle> <textur/modell> <RRGGBB | aus>
static BefehlFarbe(playerid, const params[])
{
	new index = 0, wort[16], spieler, stelle, teil[TEIL_LAENGE], farbe[16], text[144];

	if (!NaechstesWort(params, index, wort, sizeof(wort)))
	{
		return SendClientMessage(playerid, 0xFFFFFFFF, "Benutze: /farbe [ID] [Stelle 0-17] [textur/modell] [RRGGBB | aus]");
	}
	spieler = strval(wort);
	if (!NaechstesWort(params, index, wort, sizeof(wort))
		|| !NaechstesWort(params, index, teil, sizeof(teil))
		|| !NaechstesWort(params, index, farbe, sizeof(farbe)))
	{
		return SendClientMessage(playerid, 0xFFFFFFFF, "Benutze: /farbe [ID] [Stelle 0-17] [textur/modell] [RRGGBB | aus]");
	}
	stelle = strval(wort);

	if (spieler < 0 || spieler >= MAX_PLAYERS || !IsPlayerConnected(spieler))
	{
		return SendClientMessage(playerid, 0xFF4040FF, "[Kleidung] Kein Spieler mit dieser ID.");
	}
	if (stelle < 0 || stelle >= TEILE_ANZAHL)
	{
		return SendClientMessage(playerid, 0xFF4040FF, "[Kleidung] Stelle 0 bis 17.");
	}
	if (4 <= stelle <= 12)
	{
		return SendClientMessage(playerid, 0xFF4040FF, "[Kleidung] Stellen 4 bis 12 sind Tattoos, die setzt dieser Befehl nicht.");
	}

	new schraeg = strfind(teil, "/");
	new textur[TEIL_LAENGE], modell[TEIL_LAENGE];
	if (schraeg == -1)
	{
		return SendClientMessage(playerid, 0xFF4040FF, "[Kleidung] Teil als textur/modell, z. B. bandred/bandana.");
	}
	strmid(textur, teil, 0, schraeg, sizeof(textur));
	strmid(modell, teil, schraeg + 1, strlen(teil), sizeof(modell));
	if (!IstName(textur) || !IstName(modell))
	{
		return SendClientMessage(playerid, 0xFF4040FF, "[Kleidung] Teil ungueltig, nur Buchstaben, Ziffern und _.");
	}

	new bool:mit_farbe = strcmp(farbe, "aus", true) != 0;
	if (mit_farbe && !IstHex(farbe))
	{
		return SendClientMessage(playerid, 0xFF4040FF, "[Kleidung] Farbe als RRGGBB (z. B. FFDC00) oder aus.");
	}
	if (mit_farbe && stelle != 0 && stelle != 2)
	{
		return SendClientMessage(playerid, 0xFF4040FF, "[Kleidung] Farben nur an Stelle 0 (Oberteil) und 2 (Hose).");
	}

	new token[TEIL_LAENGE];
	if (mit_farbe)
	{
		format(token, sizeof(token), "%s@%s", teil, farbe);
	}
	else
	{
		format(token, sizeof(token), "%s", teil);
	}

	if (gOutfit[spieler] == -1)
	{
		strmid(gOutfitText[spieler], OUTFIT_STANDARD, 0, strlen(OUTFIT_STANDARD), OUTFIT_LAENGE);
	}
	if (!TeilSetzen(spieler, stelle, token))
	{
		return SendClientMessage(playerid, 0xFF4040FF, "[Kleidung] Outfit kaputt, erst /kleidung 1 bis 5.");
	}
	gOutfit[spieler] = 0;
	KleidungAnAlle(spieler);

	format(text, sizeof(text), "[Kleidung] Spieler %d, Stelle %d: %s an alle mit Plugin gesendet.", spieler, stelle, token);
	SendClientMessage(playerid, 0x007DFFFF, text);
	printf("[Kleidung] /farbe von %d: Spieler %d -> %s", playerid, spieler, gOutfitText[spieler]);
	if (GetPlayerSkin(spieler) != 0)
	{
		SendClientMessage(playerid, 0xFF4040FF, "[Kleidung] Der Spieler hat nicht Skin 0, man sieht es erst mit Skin 0.");
	}
	return 1;
}

static KleidungAnAlle(spieler)
{
	for (new i = 0; i < MAX_PLAYERS; i++)
	{
		if (IsPlayerConnected(i) && gPluginVersion[i][0] != '\0')
		{
			KleidungSenden(i, spieler);
		}
	}
}

static KleidungAnAlleWeg(spieler)
{
	for (new i = 0; i < MAX_PLAYERS; i++)
	{
		if (IsPlayerConnected(i) && gPluginVersion[i][0] != '\0')
		{
			cef_emit_event(i, "kleidung:weg", CEFINT(spieler));
		}
	}
}

public OnPlayerCommandText(playerid, cmdtext[])
{
	if (!strcmp(cmdtext, "/farbe", true, 6) && (cmdtext[6] == ' ' || cmdtext[6] == '\0'))
	{
		BefehlFarbe(playerid, cmdtext[6]);
		return 1;
	}

	// /kleidunginfo ist ein Befehl des Plugins im Client und kommt nur ohne Plugin hier an
	if (strcmp(cmdtext, "/kleidung", true, 9) != 0 || (cmdtext[9] != ' ' && cmdtext[9] != '\0'))
	{
		return 0;
	}

	new text[144];

	if (!strcmp(cmdtext, "/kleidung skin", true))
	{
		SetPlayerSkin(playerid, 0);
		SendClientMessage(playerid, 0x007DFFFF, "[Kleidung] Skin 0 (CJ) gesetzt.");
		return 1;
	}
	if (!strcmp(cmdtext, "/kleidung aus", true))
	{
		gOutfit[playerid] = -1;
		KleidungAnAlleWeg(playerid);
		SendClientMessage(playerid, 0x007DFFFF, "[Kleidung] Outfit vergessen, alle sehen dich als Standard-CJ.");
		return 1;
	}
	if (!strcmp(cmdtext, "/kleidung frage", true))
	{
		for (new i = 0; i < MAX_PLAYERS; i++)
		{
			if (IsPlayerConnected(i))
			{
				cef_emit_event(i, "kleidung:frage");
			}
		}
		SendClientMessage(playerid, 0x007DFFFF, "[Kleidung] Alle Clients gefragt.");
		return 1;
	}
	if (!strcmp(cmdtext, "/kleidung info", true))
	{
		new name[MAX_PLAYER_NAME], plugin[16];
		for (new i = 0; i < MAX_PLAYERS; i++)
		{
			if (!IsPlayerConnected(i))
			{
				continue;
			}
			GetPlayerName(i, name, sizeof(name));
			if (gPluginVersion[i][0] != '\0')
			{
				strmid(plugin, gPluginVersion[i], 0, strlen(gPluginVersion[i]), sizeof(plugin));
			}
			else
			{
				plugin = "fehlt";
			}
			if (gOutfit[i] == -1)
			{
				format(text, sizeof(text), "[Kleidung] %d %s: Skin %d, Plugin %s, kein Outfit", i, name, GetPlayerSkin(i), plugin);
			}
			else if (gOutfit[i] == 0)
			{
				format(text, sizeof(text), "[Kleidung] %d %s: Skin %d, Plugin %s, Outfit mit /farbe geaendert", i, name, GetPlayerSkin(i), plugin);
			}
			else
			{
				format(text, sizeof(text), "[Kleidung] %d %s: Skin %d, Plugin %s, Outfit %d", i, name, GetPlayerSkin(i), plugin, gOutfit[i]);
			}
			SendClientMessage(playerid, 0xFFFFFFFF, text);
			if (gOutfit[i] != -1)
			{
				// das Outfit in Stuecken, damit keine Zeile ueber 144 Zeichen kommt
				for (new start = 0, laenge = strlen(gOutfitText[i]); start < laenge; start += 120)
				{
					strmid(text, gOutfitText[i], start, start + 120, sizeof(text));
					SendClientMessage(playerid, 0xC0C0C0FF, text);
				}
			}
		}
		return 1;
	}

	new nummer = cmdtext[9] == ' ' ? strval(cmdtext[10]) : 0;
	if (nummer < 1 || nummer > sizeof(OUTFIT_TEILE))
	{
		SendClientMessage(playerid, 0xFFFFFFFF, "Benutze: /kleidung [skin | 1 bis 5 | aus | info | frage], Farben: /farbe");
		return 1;
	}

	gOutfit[playerid] = nummer;
	strmid(gOutfitText[playerid], OUTFIT_TEILE[nummer - 1], 0, strlen(OUTFIT_TEILE[nummer - 1]), OUTFIT_LAENGE);
	KleidungAnAlle(playerid);
	format(text, sizeof(text), "[Kleidung] Outfit %d an alle mit Plugin gesendet.", nummer);
	SendClientMessage(playerid, 0x007DFFFF, text);
	if (GetPlayerSkin(playerid) != 0)
	{
		SendClientMessage(playerid, 0xFF4040FF, "[Kleidung] Du hast nicht Skin 0, man sieht es erst mit /kleidung skin.");
	}
	return 1;
}
