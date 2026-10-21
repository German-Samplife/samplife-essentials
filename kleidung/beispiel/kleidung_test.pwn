// Filterscript zum Testen des Kleidungs-Plugins mit dem Server (siehe kleidung/README.md).
// Braucht die samp-cef-Server-Komponente (jede ab f14fe4f) und deren cef.inc. Beim
// Spieler: samp-cef-Client ab 0x000300 (cef/plugin-ereignisse.patch) und
// samplife-kleidung.dll in <GTA>/cef/plugins/.
//
// Spieler ohne Skin 0 sehen nichts davon; vorher z. B. /kleidung skin.
//
// Befehle:
//   /kleidung skin   eigenen Skin auf 0 (CJ) setzen
//   /kleidung 1      Outfit 1 (schwarzes Unterhemd, Afro, Tarnhose, Cowboystiefel, Kette, Brille)
//   /kleidung 2      Outfit 2 (roter Anzug, Glatze, Anzughose, schwarze Schuhe, Uhr, Cowboyhut)
//   /kleidung 3      Outfit 3 (Hoodie, Jogginghose, Turnschuhe, Cap, Tattoo am Arm)
//   /kleidung aus    eigenes Outfit vergessen, alle sehen den Standard-CJ
//   /kleidung info   wer das Plugin gemeldet hat und wer welches Outfit trägt
//   /kleidung frage  alle Clients fragen, ob das Plugin läuft (Antwort kleidung:hallo)

#define FILTERSCRIPT

#include <open.mp>
#include <cef>

// 18 Teile in der Reihenfolge von eClothesTexturePart: Oberkörper, Kopf, Beine,
// Schuhe, neun Tattoo-Stellen, Kette, Uhr, Brille, Hut, Spezial. Namen wie in der
// Kleidungsliste von MTA (CClientPlayerClothes.cpp); Tattoos beginnen mit ihrer Stelle
// (4weed an Stelle 4). textur/modell, bei Tattoos nur textur, "-" für nichts.
// Chatzeilen ohne Umlaute: Diese Datei ist UTF-8, SA-MP erwartet Windows-1252.
static const OUTFIT_TEILE[3][] =
{
	"vestblack/vest afro/afro worktrcamogrn/worktr cowboyboot/biker - - - - - - - - - neckgold/neck2 - glasses04dark/glasses04 - -",
	"suit1red/suit1 bald/head suit1trred/suit1tr shoedressblk/shoe - - - - - - - - - - watchgno/watch - cowboy/cowboy -",
	"hoodyabase5/hoodya player_face/head tracktrgang/tracktr sneakerbincblk/sneaker 4weed - - - - - - - - - - - capblk/cap -"
};

static gOutfit[MAX_PLAYERS] = {-1, ...};
static gPluginVersion[MAX_PLAYERS][16];

public OnFilterScriptInit()
{
	cef_subscribe("kleidung:hallo", "KleidungHallo");
	print("[Kleidung] Test-Filterscript geladen");
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

static KleidungSenden(empfaenger, spieler)
{
	cef_emit_event(empfaenger, "kleidung:spieler", CEFINT(spieler), CEFSTR(OUTFIT_TEILE[gOutfit[spieler]]));
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
			format(text, sizeof(text), "[Kleidung] %d %s: Skin %d, Plugin %s, Outfit %d", i, name, GetPlayerSkin(i), plugin, gOutfit[i] + 1);
			SendClientMessage(playerid, 0xFFFFFFFF, text);
		}
		return 1;
	}

	new nummer = cmdtext[9] == ' ' ? strval(cmdtext[10]) : 0;
	if (nummer < 1 || nummer > sizeof(OUTFIT_TEILE))
	{
		SendClientMessage(playerid, 0xFFFFFFFF, "Benutze: /kleidung [skin | 1 | 2 | 3 | aus | info | frage]");
		return 1;
	}

	gOutfit[playerid] = nummer - 1;
	KleidungAnAlle(playerid);
	format(text, sizeof(text), "[Kleidung] Outfit %d an alle mit Plugin gesendet.", nummer);
	SendClientMessage(playerid, 0x007DFFFF, text);
	if (GetPlayerSkin(playerid) != 0)
	{
		SendClientMessage(playerid, 0xFF4040FF, "[Kleidung] Du hast nicht Skin 0, man sieht es erst mit /kleidung skin.");
	}
	return 1;
}
