import { makeAuthenticatedRequest } from '$lib/api';

/** @type {import('./$types').PageServerLoad} */
export async function load({ params, fetch, cookies }) {
    let res = await makeAuthenticatedRequest(
        `api/draw/${params.round_id}`,
        cookies,
        {}
    )
    const presentationInfo = await res.json();

    return {
        info: presentationInfo,
        tournamentId: params.tournament_id,
        roundId: params.round_id,
    };
}