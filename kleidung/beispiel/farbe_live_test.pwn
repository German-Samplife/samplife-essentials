// Farben-Test für einen laufenden Server mit Gamemode (samplife-kleidung ab 0.9.0). Anders als kleidung_test.pwn
// abonniert dieses Filterscript keine samp-cef-Ereignisse: Der Server-Teil von samp-cef merkt sich je Ereignis nur ein
// Script (cef_subscribe überschreibt), ein zweites Abo von kleidung:hallo nähme dem Gamemode die Anmeldung der Plugins
// weg, bis zum nächsten Neustart. Nur für eingeloggte RCON-Admins.
//
// /farbe [ID] [Stelle] [textur/modell] [RRGGBB | aus]
//   Setzt beim Spieler das Teil an der Stelle (0 bis 17, ohne Tattoos 4 bis 12), mit Farbe oder ohne. Die übrigen Stellen
//   sind der Standard-CJ bzw. was dieses Script dem Spieler schon gesetzt hat. Geht an alle Clients; Plugins vor 0.9.0
//   verwerfen gefärbte Outfits mit einer Logzeile, das ist für den Test harmlos. Der Gamemode setzt das eigene Outfit
//   wieder, sobald er es neu schickt (z. B. nach dem nächsten Login).
//
// Laden: /rcon loadfs farbe_live_test, entladen: /rcon unloadfs farbe_live_test

#include <open.mp>
#include <cef>
#include <sscanf2>

#define TEILE 18
#define OUTFIT_LAENGE 400

static gTeile[MAX_PLAYERS][TEILE][40];
static bool:gHat[MAX_PLAYERS];

static const STANDARD[TEILE][] =
{
	"vest/vest", "player_face/head", "jeansdenim/jeans", "sneakerbincblk/sneaker", "-", "-", "-", "-", "-", "-", "-", "-",
	"-", "-", "-", "-", "-", "-"
};

public OnFilterScriptInit()
{
	print("[Farbe] Live-Test geladen: /farbe nur für RCON-Admins");
	return 1;
}

public OnPlayerConnect(playerid)
{
	gHat[playerid] = false;
	return 1;
}

public OnPlayerDisconnect(playerid, reason)
{
	gHat[playerid] = false;
	return 1;
}

static bool:NameOk(const name[])
{
	new n = strlen(name);
	if (n < 3 || n > 32 || strfind(name, "/") < 1) return false;
	for (new i = 0; i < n; i++)
	{
		new c = name[i];
		if (!((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_' || c == '/')) return false;
	}
	return true;
}

static bool:FarbeOk(const farbe[])
{
	if (strlen(farbe) != 6) return false;
	for (new i = 0; i < 6; i++)
	{
		new c = farbe[i];
		if (!((c >= '0' && c <= '9') || (c >= 'A' && c <= 'F') || (c >= 'a' && c <= 'f'))) return false;
	}
	return true;
}

public OnPlayerCommandText(playerid, cmdtext[])
{
	if (strcmp(cmdtext, "/farbe", true, 6) != 0 || (cmdtext[6] != ' ' && cmdtext[6] != '\0')) return 0;
	if (!IsPlayerAdmin(playerid)) return 0;

	new id, stelle, teil[40], farbe[16];
	if (sscanf(cmdtext[6], "iis[40]s[16]", id, stelle, teil, farbe))
		return SendClientMessage(playerid, 0xFFFFFFFF, "Benutze: /farbe [ID] [Stelle 0-17] [textur/modell] [RRGGBB | aus]");
	if (!IsPlayerConnected(id)) return SendClientMessage(playerid, 0xFF4040FF, "[Farbe] Kein Spieler mit dieser ID.");
	if (stelle < 0 || stelle >= TEILE || (stelle >= 4 && stelle <= 12))
		return SendClientMessage(playerid, 0xFF4040FF, "[Farbe] Stelle 0 bis 17 ohne Tattoos 4 bis 12.");
	if (!NameOk(teil)) return SendClientMessage(playerid, 0xFF4040FF, "[Farbe] Teil als textur/modell, z. B. bandred/bandana.");
	new bool:aus = !strcmp(farbe, "aus", true);
	if (!aus && !FarbeOk(farbe)) return SendClientMessage(playerid, 0xFF4040FF, "[Farbe] Farbe als RRGGBB (z. B. FFDC00) oder aus.");

	if (!gHat[id])
	{
		for (new t = 0; t < TEILE; t++) format(gTeile[id][t], sizeof(gTeile[][]), "%s", STANDARD[t]);
		gHat[id] = true;
	}
	if (aus) format(gTeile[id][stelle], sizeof(gTeile[][]), "%s", teil);
	else format(gTeile[id][stelle], sizeof(gTeile[][]), "%s@%s", teil, farbe);

	new outfit[OUTFIT_LAENGE];
	for (new t = 0; t < TEILE; t++)
	{
		if (t > 0) strcat(outfit, " ");
		strcat(outfit, gTeile[id][t]);
	}
	for (new i = 0; i < MAX_PLAYERS; i++)
	{
		if (IsPlayerConnected(i)) cef_emit_event(i, "kleidung:spieler", CEFINT(id), CEFSTR(outfit));
	}
	new text[144];
	format(text, sizeof(text), "[Farbe] Spieler %d, Stelle %d: %s", id, stelle, gTeile[id][stelle]);
	SendClientMessage(playerid, 0x007DFFFF, text);
	printf("[Farbe] %d setzt bei %d: %s", playerid, id, outfit);
	return 1;
}
