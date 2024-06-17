import { env } from '$env/dynamic/public'
import { getParticipantIdInTournamentServerOnly, makeAuthenticatedRequestServerOnly } from '$lib/api';
import { redirect } from '@sveltejs/kit';

/** @type {import('./$types').PageServerLoad} */
export async function load({ params, fetch, cookies }) {
    let participantId = getParticipantIdInTournamentServerOnly(cookies, params.tournament_id);
    let additionalLinks = [];

    let tournamentName = "";
    if (participantId !== null) {
        let res = await makeAuthenticatedRequestServerOnly(
            `api/participant/${participantId}/info`,
            cookies,
            {}
        )
        const participantInfo = await res.json();

        additionalLinks.push(
            {
                name: "Overview",
                url: `/tournament/${params.tournament_id}/home`,
            }
        )

        additionalLinks.push(
            {
                name: "Tab",
                url: `/tournament/${params.tournament_id}/tab`,
            }
        )

        additionalLinks.push(
            {
                name: "Settings",
                url: `/tournament/${params.tournament_id}/settings`,
            }
        )
    
        if (participantInfo.role.type == "Adjudicator") {
            additionalLinks.push({
                name: "Feedback",
                url: `/tournament/${params.tournament_id}/home/${participantId}/feedback`,
            });
        }

        tournamentName = participantInfo.tournament_name;
    }
    else {
        let public_info = await makeAuthenticatedRequestServerOnly(
            `api/tournament/${params.tournament_id}/public`,
            cookies,
            {}
        );
        public_info = await public_info.json();

        additionalLinks.push(
            {
                name: "Overview",
                url: `/tournament/${params.tournament_id}/public`,
            }
        );

        if (public_info.show_tab) {
            additionalLinks.push(
                {
                    name: "Tab",
                    url: `/tournament/${params.tournament_id}/tab`,
                }
            );
        }

        tournamentName = public_info.tournament_name;
    }


    return {
        additionalLinks,
        tournamentId: params.tournament_id,
        tournamentName,
    };
}