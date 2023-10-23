import { make_authenticated_request } from '$lib/api';

/** @type {import('./$types').PageServerLoad} */
export async function load({ params, fetch, cookies }) {
    let res = await make_authenticated_request(
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