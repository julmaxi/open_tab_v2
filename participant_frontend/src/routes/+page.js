import { makeRequest } from "$lib/api";

export async function load({ params, fetch }) {
    console.log("HJKJKH");
    let request = await makeRequest(fetch, 'api/public_tournaments', {
        method: 'GET'
    });

    let tournamentsInfo = await request.json();
    console.log(">>", tournamentsInfo, request);

    return {
        tournamentsInfo
    };
}