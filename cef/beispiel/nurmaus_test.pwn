// Filterscript zum Testen des Eingabemodus "nur Maus" (siehe cef/TESTEN.md).
// Braucht die Server-Komponente aus samp-cef f14fe4f + cef/nur-maus.patch
// und deren server/cef.inc. Die Seiten index.html und tasten.html liegen beim
// Spieler unter <GTA>/cef/assets/nur-maus-test/.
//
// Befehle:
//   /nurmaus         beide Seiten öffnen, Testseite im Modus mouse
//   /nurmaus voll    Testseite auf full
//   /nurmaus maus    Testseite auf mouse
//   /nurmaus aus     Testseite auf none (bleibt sichtbar)
//   /nurmaus alt     Testseite mit cef_focus_browser(.., true) wie bisher
//   /nurmaus weg     beide Seiten schließen

#define FILTERSCRIPT

#include <open.mp>
#include <cef>

#define NURMAUS_BROWSER 9101
#define TASTEN_BROWSER  9102

#define NURMAUS_URL "nur-maus-test/index.html"
#define TASTEN_URL  "nur-maus-test/tasten.html"

public OnPlayerCommandText(playerid, cmdtext[])
{
	if (!strcmp(cmdtext, "/nurmaus", true))
	{
		new version = cef_get_client_version(playerid);
		new text[112];
		format(text, sizeof(text), "[CEF] Client-Version %x", version);
		SendClientMessage(playerid, 0x007DFFFF, text);
		if (version < CEF_CLIENT_VERSION_INPUT_MODE)
		{
			SendClientMessage(playerid, 0xFF4040FF, "[CEF] Client zu alt: mouse wird dort zu full.");
		}

		cef_create_browser(playerid, TASTEN_BROWSER, TASTEN_URL, false, false);
		cef_always_listen_keys(playerid, TASTEN_BROWSER, true);
		cef_create_browser(playerid, NURMAUS_BROWSER, NURMAUS_URL, false, false);
		cef_set_input_mode(playerid, NURMAUS_BROWSER, CEF_INPUT_MOUSE_ONLY);
		return 1;
	}
	if (!strcmp(cmdtext, "/nurmaus voll", true))
	{
		cef_set_input_mode(playerid, NURMAUS_BROWSER, CEF_INPUT_FULL);
		return 1;
	}
	if (!strcmp(cmdtext, "/nurmaus maus", true))
	{
		cef_set_input_mode(playerid, NURMAUS_BROWSER, CEF_INPUT_MOUSE_ONLY);
		return 1;
	}
	if (!strcmp(cmdtext, "/nurmaus aus", true))
	{
		cef_set_input_mode(playerid, NURMAUS_BROWSER, CEF_INPUT_NONE);
		return 1;
	}
	if (!strcmp(cmdtext, "/nurmaus alt", true))
	{
		cef_focus_browser(playerid, NURMAUS_BROWSER, true);
		return 1;
	}
	if (!strcmp(cmdtext, "/nurmaus weg", true))
	{
		cef_destroy_browser(playerid, NURMAUS_BROWSER);
		cef_destroy_browser(playerid, TASTEN_BROWSER);
		return 1;
	}
	return 0;
}

public OnCefInputModeChanged(player_id, browser_id, mode)
{
	new text[96];
	switch (mode)
	{
		case CEF_INPUT_NONE: format(text, sizeof(text), "[CEF] Browser %d: Eingabemodus none", browser_id);
		case CEF_INPUT_FULL: format(text, sizeof(text), "[CEF] Browser %d: Eingabemodus full", browser_id);
		case CEF_INPUT_MOUSE_ONLY: format(text, sizeof(text), "[CEF] Browser %d: Eingabemodus mouse", browser_id);
		default: format(text, sizeof(text), "[CEF] Browser %d: Eingabemodus %d", browser_id, mode);
	}
	SendClientMessage(player_id, 0x007DFFFF, text);
	return 1;
}
