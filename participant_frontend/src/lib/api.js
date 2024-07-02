import { browser } from "$app/environment";
import { env } from "$env/dynamic/public";

export async function makeAuthenticatedRequestServerOnly(
    url,
    cookies,
    options
) {
    let headers = {'Authorization': `Bearer ${cookies.get("token")}`};
    if (options.headers !== undefined) {
        headers = {...headers, ...options.headers};
        options.headers = undefined;
    }

    if (env.PUBLIC_API_URL === undefined) {
        throw Error("PUBLIC_API_URL is undefined");
    }

    const res = await fetch(
        `${env.PUBLIC_API_URL}/${url}`,
        {
            ...options,
            headers: new Headers(headers),
        }
    );
    if (res.status != 200) {
        console.log(Error(`Request to ${url} failed with status ${res.status}: ${await res.text()}`));
        throw Error(`Request to ${url} failed with status ${res.status}: ${await res.text()}`);
    }
    return res   
}

export function getParticipantCookieNameInTournament(tournamentId) {
    return "participant_id:" + tournamentId;
}


export function getParticipantIdInTournamentServerOnly(cookies, tournamentId) {
    let participantId = cookies.get(getParticipantCookieNameInTournament(tournamentId));
    if (participantId === undefined) {
        return null;
    }
    return participantId;
}


export function getParticipantIdInTournament(tournamentId) {
    //Read cookie
    let cookies = document.cookie.split("; ");
    let participantId = undefined;
    for (let i = 0; i < cookies.length; i++) {
        let cookie = cookies[i].split("=");
        if (cookie[0] === getParticipantCookieNameInTournament(tournamentId)) {
            participantId = cookie[1];
            break;
        }
    }
    if (participantId === undefined) {
        throw Error("participant_id cookie is undefined");
    }
    return participantId;
}

export async function getToken() {
    let token = undefined;
    if (sessionStorage.getItem("token") === null || sessionStorage.getItem("tokenExpires") === null || Date.now() + 1000 > parseInt(sessionStorage.getItem("tokenExpires"))) {
        let tokenRequest = await fetch(
            "/auth",
            {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
            }
        );

        let response = await tokenRequest.json();

        token = response.token;
        sessionStorage.setItem("token", response.token);
        sessionStorage.setItem("tokenExpires", response.expires);
    }
    else {
        token = sessionStorage.getItem("token");
    }

    return token;
}

export async function makeAuthenticatedRequest(
    fetch,
    url,
    options
) {
    if (env.PUBLIC_API_URL === undefined) {
        throw Error("PUBLIC_API_URL is undefined");
    }
    

    let authString = "missing"; // This will be filled in by the hook on the server
    //Check if we are in the browser and if there is a token in the session storage
    if (browser) {
        let token = await getToken();
        authString = `Bearer ${token}`;
    }

    const res = await fetch(
        `${env.PUBLIC_API_URL}/${url}`,
        {
            ...options,
            headers: new Headers({'Authorization': authString}),
        }
    );
    if (res.status != 200) {
        console.log(Error(`Request to ${url} failed with status ${res.status}: ${await res.text()}`));
        throw Error(`Request to ${url} failed with status ${res.status}: ${await res.text()}`);
    }
    return res
}


export async function makeRequest(
    fetch,
    url,
    options
) {
    if (env.PUBLIC_API_URL === undefined) {
        throw Error("PUBLIC_API_URL is undefined");
    }

    const res = await fetch(
        `${env.PUBLIC_API_URL}/${url}`,
        {
            headers: {
                "Content-Type": "application/json",
            },
            method: "POST",
            ...options,
        }
    );
    return res
}