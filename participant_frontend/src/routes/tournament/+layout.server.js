import { env } from '$env/dynamic/public'
import { make_authenticated_request } from '$lib/api';
import { redirect } from '@sveltejs/kit';

/** @type {import('./$types').PageServerLoad} */
export async function load({ params, fetch, cookies }) {
    let participantId = cookies.get("participant_id");
    let res = await make_authenticated_request(
        `api/participant/${participantId}/info`,
        cookies,
        {}
    )
    const participantInfo = await res.json();
    let additionalLinks = [];

    if (participantInfo.role.type == "Adjudicator") {
        additionalLinks.push({
            name: "Feedback",
            url: `/tournament/${params.tournament_id}/home/${participantId}/feedback`,
        });
    }

    return {
        additionalLinks,
        tournamentId: params.tournament_id,
    };
}