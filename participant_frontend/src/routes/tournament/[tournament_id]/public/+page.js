import { makeRequest } from "$lib/api";
import { env } from '$env/dynamic/public';
import { fail } from "@sveltejs/kit";


/** @type {import('./$types').PageLoad} */
export async function load({ params, fetch }) {
    let public_info = await fetch(
        `${env.PUBLIC_API_URL}/api/tournament/${params.tournament_id}/public`);
    if (public_info.status !== 200) {
        if (public_info.status === 404) {
            throw fail(404, { message: "Tournament not found" });
        }
        else {
            throw fail(500, { message: "Failed to load tournament data" });
        }
    }
    let data = await public_info.json();

    let awards = await fetch(
        `${env.PUBLIC_API_URL}/api/tournament/${params.tournament_id}/awards`);
    if (awards.status !== 200) {
        throw fail(500, { message: "Failed to load awards data" });
    }
    let awards_data = await awards.json();
    data.awards = awards_data.awards;
    return {
        tournamentId: params.tournament_id,
        ...data
    };
}