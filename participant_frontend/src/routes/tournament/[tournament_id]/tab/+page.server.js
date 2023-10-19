import { env } from '$env/dynamic/public'
import { make_authenticated_request } from '$lib/api';

/** @type {import('./$types').PageServerLoad} */
export async function load({ params, fetch, cookies }) {
    let res = await make_authenticated_request(
        `api/tournament/${params.tournament_id}/tab`,
        cookies,
        {}
    )
    const tab = await res.json();

    return tab;
}