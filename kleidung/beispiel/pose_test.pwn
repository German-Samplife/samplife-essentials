// Filterscript zum Testen der Posen (Teilanimationen) des Plugins (siehe kleidung/README.md).
// Braucht wie kleidung_test.pwn die samp-cef-Server-Komponente und deren cef.inc. Beim
// Spieler: samp-cef-Client ab 0x000300 und samplife-kleidung.dll ab 0.6.0 in <GTA>/cef/plugins/.
//
// Laden per RCON: /rcon loadfs pose_test, entladen: /rcon unloadfs pose_test (beendet alle Posen).
// Jede Pose geht an alle verbundenen Clients, auch an die, die den Spieler gerade nicht
// sehen; das Plugin merkt sich die Pose und spielt sie, sobald der Spieler hereinkommt.
//
// Befehle:
//   /pose halten   Pose handy_halten (Haende vorn, Blick nach unten)
//   /pose funk     Pose handy_funk (Hand an der Schulter)
//   /pose aus      eigene Pose beenden
//   /pose info     wer gerade welche Pose hat

#define FILTERSCRIPT

#include <open.mp>
#include <cef>

static gPose[MAX_PLAYERS][24];

static PoseAnAlle(spieler)
{
	for (new i = 0; i < MAX_PLAYERS; i++)
	{
		if (!IsPlayerConnected(i)) continue;

		if (gPose[spieler][0])
		{
			cef_emit_event(i, "anim:start", CEFINT(spieler), CEFSTR(gPose[spieler]));
		}
		else
		{
			cef_emit_event(i, "anim:stopp", CEFINT(spieler));
		}
	}
}

public OnFilterScriptInit()
{
	print("[Pose] Test-Filterscript geladen: /pose halten, /pose funk, /pose aus, /pose info");
	return 1;
}

public OnFilterScriptExit()
{
	for (new i = 0; i < MAX_PLAYERS; i++)
	{
		if (gPose[i][0])
		{
			gPose[i][0] = EOS;
			PoseAnAlle(i);
		}
	}
	return 1;
}

public OnPlayerConnect(playerid)
{
	gPose[playerid][0] = EOS;

	// der Neue bekommt die Posen der anderen
	for (new i = 0; i < MAX_PLAYERS; i++)
	{
		if (i != playerid && IsPlayerConnected(i) && gPose[i][0])
		{
			cef_emit_event(playerid, "anim:start", CEFINT(i), CEFSTR(gPose[i]));
		}
	}
	return 1;
}

public OnPlayerDisconnect(playerid, reason)
{
	if (gPose[playerid][0])
	{
		gPose[playerid][0] = EOS;
		PoseAnAlle(playerid);
	}
	return 1;
}

public OnPlayerCommandText(playerid, cmdtext[])
{
	if (strcmp(cmdtext, "/pose", true, 5) != 0 || (cmdtext[5] != ' ' && cmdtext[5] != EOS)) return 0;

	if (!strcmp(cmdtext, "/pose halten", true))
	{
		format(gPose[playerid], sizeof(gPose[]), "handy_halten");
	}
	else if (!strcmp(cmdtext, "/pose funk", true))
	{
		format(gPose[playerid], sizeof(gPose[]), "handy_funk");
	}
	else if (!strcmp(cmdtext, "/pose aus", true))
	{
		gPose[playerid][0] = EOS;
	}
	else if (!strcmp(cmdtext, "/pose info", true))
	{
		new zeile[96], name[MAX_PLAYER_NAME], anzahl;
		for (new i = 0; i < MAX_PLAYERS; i++)
		{
			if (IsPlayerConnected(i) && gPose[i][0])
			{
				GetPlayerName(i, name, sizeof(name));
				format(zeile, sizeof(zeile), "[Pose] %s (ID %d): %s", name, i, gPose[i]);
				SendClientMessage(playerid, -1, zeile);
				anzahl++;
			}
		}
		if (!anzahl) SendClientMessage(playerid, -1, "[Pose] Niemand hat gerade eine Pose.");
		return 1;
	}
	else
	{
		SendClientMessage(playerid, -1, "[Pose] /pose halten | funk | aus | info");
		return 1;
	}

	PoseAnAlle(playerid);

	new zeile[64];
	format(zeile, sizeof(zeile), "[Pose] %s", gPose[playerid][0] ? gPose[playerid] : ("aus"));
	SendClientMessage(playerid, -1, zeile);
	return 1;
}
