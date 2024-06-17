import { makeRequest } from "$lib/api";

export async function load({ params, fetch, cookies }) {
    let public_info = await makeRequest(fetch, `api/tournament/${params.tournament_id}/public`, {method: "GET"});

    let data = await public_info.json();
    return {
        tournamentId: params.tournament_id,
        ...data
    };
}