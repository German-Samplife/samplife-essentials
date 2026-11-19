// Filterscript zum Testen des ESC-Menüs im Kleidungs-Plugin (ab 0.10.0, siehe kleidung/README.md).
// Braucht die samp-cef-Server-Komponente und deren cef.inc. Beim Spieler: samp-cef-Client
// ab 0x000300 und samplife-kleidung.dll ab 0.10.0 in <GTA>/cef/plugins/.
//
// Laden per RCON: /rcon loadfs esc_test, entladen: /rcon unloadfs esc_test (schaltet bei
// allen das ESC-Menü wieder aus). Es abonniert nur esc:bereit und esc:gedrueckt, die kein
// Gamemode benutzt; es läuft also auch neben dem Gamemode.
//
// Eine Seite gibt es hier nicht: Das Script tut so, als ginge sie auf und zu, und schreibt
// es in den Chat.
//
// Befehle:
//   /esctest an        ESC geht an den Server (esc:an), Antwort esc:bereit im Chat
//   /esctest aus       ESC öffnet wieder das GTA-Menü (esc:aus)
//   /esctest antwort   Antwort auf ESC durchschalten: offen/zu im Wechsel (Standard),
//                      keine (nach 1,5 Sekunden öffnet das Plugin das GTA-Menü)
//   /esctest original  GTA-Menü öffnen (esc:original), wie der Eintrag "GTA-Einstellungen"
//
// Chatzeilen ohne Umlaute: Diese Datei ist UTF-8, SA-MP erwartet Windows-1252.

#define FILTERSCRIPT

#include <open.mp>
#include <cef>

#define FARBE_INFO 0x007DFFFF
#define FARBE_FEHLER 0xFF4040FF

// gOffen: die gespielte Seite ist offen; gKeineAntwort: auf ESC nicht antworten
static bool:gOffen[MAX_PLAYERS];
static bool:gKeineAntwort[MAX_PLAYERS];

public OnFilterScriptInit()
{
	// Nicht hier abonnieren: Per loadfs nachgeladen kennt open.mp das Script in
	// OnFilterScriptInit noch nicht, cef_subscribe findet es nicht und tut nichts.
	SetTimer("EscAbonnieren", 0, false);
	print("[ESC] Test-Filterscript geladen, /esctest an");
	return 1;
}

forward EscAbonnieren();
public EscAbonnieren()
{
	new bereit = cef_subscribe("esc:bereit", "EscBereit");
	new gedrueckt = cef_subscribe("esc:gedrueckt", "EscGedrueckt");
	printf("[ESC] abonniert: esc:bereit %d, esc:gedrueckt %d", bereit, gedrueckt);
	return 1;
}

public OnFilterScriptExit()
{
	for (new i = 0; i < MAX_PLAYERS; i++)
	{
		if (IsPlayerConnected(i))
		{
			cef_emit_event(i, "esc:aus");
		}
	}
	return 1;
}

public OnPlayerConnect(playerid)
{
	gOffen[playerid] = false;
	gKeineAntwort[playerid] = false;
	return 1;
}

forward EscBereit(player_id, const arguments[]);
public EscBereit(player_id, const arguments[])
{
	if (!strcmp(arguments, "1"))
	{
		SendClientMessage(player_id, FARBE_INFO, "[ESC] Plugin bereit: ESC geht jetzt an den Server.");
	}
	else
	{
		SendClientMessage(player_id, FARBE_FEHLER, "[ESC] Plugin kann ESC nicht abfangen, Grund in samplife-kleidung.log.");
	}
	printf("[ESC] Spieler %d: esc:bereit %s", player_id, arguments);
	return 1;
}

forward EscGedrueckt(player_id, const arguments[]);
public EscGedrueckt(player_id, const arguments[])
{
	printf("[ESC] Spieler %d: esc:gedrueckt", player_id);

	if (gKeineAntwort[player_id])
	{
		SendClientMessage(player_id, FARBE_INFO, "[ESC] ESC gefangen, keine Antwort: gleich kommt das GTA-Menu.");
		return 1;
	}

	gOffen[player_id] = !gOffen[player_id];

	if (gOffen[player_id])
	{
		cef_emit_event(player_id, "esc:offen");
		SendClientMessage(player_id, FARBE_INFO, "[ESC] ESC gefangen: Seite waere jetzt offen. Nochmal ESC schliesst sie.");
	}
	else
	{
		cef_emit_event(player_id, "esc:zu");
		SendClientMessage(player_id, FARBE_INFO, "[ESC] ESC gefangen: Seite waere jetzt zu.");
	}
	return 1;
}

public OnPlayerCommandText(playerid, cmdtext[])
{
	if (strcmp(cmdtext, "/esctest", true, 8) != 0 || (cmdtext[8] != ' ' && cmdtext[8] != '\0'))
	{
		return 0;
	}

	if (!strcmp(cmdtext, "/esctest an", true))
	{
		EscAbonnieren();
		gOffen[playerid] = false;
		cef_emit_event(playerid, "esc:an");
		SendClientMessage(playerid, FARBE_INFO, "[ESC] esc:an gesendet, warte auf esc:bereit.");
		return 1;
	}

	if (!strcmp(cmdtext, "/esctest aus", true))
	{
		gOffen[playerid] = false;
		cef_emit_event(playerid, "esc:aus");
		SendClientMessage(playerid, FARBE_INFO, "[ESC] esc:aus gesendet, ESC oeffnet wieder das GTA-Menu.");
		return 1;
	}

	if (!strcmp(cmdtext, "/esctest antwort", true))
	{
		gKeineAntwort[playerid] = !gKeineAntwort[playerid];
		SendClientMessage(playerid, FARBE_INFO, gKeineAntwort[playerid]
			? ("[ESC] Antwort auf ESC: keine (Rueckfall auf das GTA-Menu nach 1,5 Sekunden).")
			: ("[ESC] Antwort auf ESC: offen und zu im Wechsel."));
		return 1;
	}

	if (!strcmp(cmdtext, "/esctest original", true))
	{
		gOffen[playerid] = false;
		cef_emit_event(playerid, "esc:original");
		SendClientMessage(playerid, FARBE_INFO, "[ESC] esc:original gesendet, das GTA-Menu geht auf.");
		return 1;
	}

	SendClientMessage(playerid, 0xFFFFFFFF, "Benutze: /esctest [an | aus | antwort | original]");
	return 1;
}
