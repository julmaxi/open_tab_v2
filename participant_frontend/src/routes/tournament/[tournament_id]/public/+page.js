import { makeRequest } from "$lib/api";
import { env } from '$env/dynamic/public';

export async function load({ params, fetch, cookies }) {
    let public_info = await fetch(
        `${env.PUBLIC_API_URL}/api/tournament/${params.tournament_id}/public`);
    let data = await public_info.json();
    return {
        tournamentId: params.tournament_id,
        ...data
    };
}