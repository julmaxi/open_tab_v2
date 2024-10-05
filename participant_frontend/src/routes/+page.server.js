import { makeRequest, makeAuthenticatedRequest, makeAuthenticatedRequestServerOnly } from "$lib/api";

export async function load({ params, fetch, cookies }) {
    let request = null;
    if (!cookies.get("token")) {
        request = await makeRequest(fetch, 'api/public_tournaments', {
            method: 'GET'
        });
    }
    else {
        request = await makeAuthenticatedRequestServerOnly('api/public_tournaments', cookies, {
            method: 'GET'
        });
    }

    let tournamentsInfo = await request.json();

    return {
        tournamentsInfo,
    };
}