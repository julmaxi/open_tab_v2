import { env } from "$env/dynamic/public";

export async function makeAuthenticatedRequest(
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


export function getParticipantIdInTournament(cookies, tournamentId) {
    let participantId = cookies.get(getParticipantCookieNameInTournament(tournamentId));
    if (participantId === undefined) {
        throw Error("participant_id cookie is undefined");
    }
    return participantId;
}