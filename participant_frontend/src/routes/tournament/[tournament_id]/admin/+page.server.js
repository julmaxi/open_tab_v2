import { env } from '$env/dynamic/public'
import { makeAuthenticatedRequestServerOnly } from '$lib/api';
import { redirect } from '@sveltejs/kit';

/** @type {import('./$types').PageLoad} */
export async function load({ params, fetch, cookies }) {    
    let clashReq = await makeAuthenticatedRequestServerOnly(
        `api/tournament/${params.tournament_id}/admin`,
        cookies,
        {}
    );
    let { rounds, tournament_name } = await clashReq.json();

    return {
        rounds,
        tournament_name,
        tournament_id: params.tournament_id
    };
}
