import { makeRequest } from "$lib/api";

export async function load({ params, fetch }) {
    let request = await makeRequest(fetch, 'api/public_tournaments', {
        method: 'GET'
    });

    let tournamentsInfo = await request.json();

    return {
        tournamentsInfo
    };
}