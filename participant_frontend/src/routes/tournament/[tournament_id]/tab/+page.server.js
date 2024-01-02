import { env } from '$env/dynamic/public'
import { makeAuthenticatedRequestServerOnly } from '$lib/api';

/** @type {import('./$types').PageServerLoad} */
export async function load({ params, fetch, cookies }) {
    let res = await makeAuthenticatedRequestServerOnly(
        `api/tournament/${params.tournament_id}/tab`,
        cookies,
        {}
    )
    const tab = await res.json();

    return tab;
}