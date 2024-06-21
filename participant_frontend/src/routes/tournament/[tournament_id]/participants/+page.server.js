import { makeAuthenticatedRequestServerOnly } from '$lib/api';

/** @type {import('./$types').PageServerLoad} */
export async function load({ params, fetch, cookies }) {
    let res = await makeAuthenticatedRequestServerOnly(
        `api/tournament/${params.tournament_id}/participants`,
        cookies,
        {}
    )
    const participants = await res.json();

    return participants;
}