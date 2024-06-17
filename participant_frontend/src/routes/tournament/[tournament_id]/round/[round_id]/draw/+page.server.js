import { env } from '$env/dynamic/public'
import { makeAuthenticatedRequestServerOnly } from '$lib/api';

/** @type {import('./$types').PageServerLoad} */
export async function load({ params, fetch, cookies }) {
    let res = await makeAuthenticatedRequestServerOnly(
        `api/rounds/${params.round_id}/draw`,
        cookies,
        {}
    )
    const draw = await res.json();

    return draw;
}