import { env } from '$env/dynamic/public'
import { makeAuthenticatedRequest } from '$lib/api';

/** @type {import('./$types').PageServerLoad} */
export async function load({ params, fetch, cookies }) {
    let res = await makeAuthenticatedRequest(
        `api/tournament/${params.tournament_id}/tab`,
        cookies,
        {}
    )
    const tab = await res.json();

    return tab;
}