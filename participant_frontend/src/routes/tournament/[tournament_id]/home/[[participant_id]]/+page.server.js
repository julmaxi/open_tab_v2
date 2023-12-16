import { env } from '$env/dynamic/public'
import { makeAuthenticatedRequest } from '$lib/api';
import { redirect } from '@sveltejs/kit';

/** @type {import('./$types').PageServerLoad} */
export async function load({ params, fetch, cookies }) {
    /*const res = await fetch(
        `${env.PUBLIC_API_URL}/api/participant/${params.participant_id}`,
        {
            headers: new Headers({'Authorization': `Bearer ${cookies.get("token")}`}),
        }
    );*/
    if (params.participant_id === undefined) {
        throw redirect(302, `/tournament/${params.tournament_id}/home/${cookies.get("participant_id")}`);
    }
    let res = await makeAuthenticatedRequest(
        `api/participant/${params.participant_id}`,
        cookies,
        {}
    )
    const participant_info = await res.json();

    return {
        name: participant_info.name,
        rounds: participant_info.rounds,
        feedback_submissions: participant_info.feedback_submissions,
        tournamentId: params.tournament_id,
        role: participant_info.role,
    };
}