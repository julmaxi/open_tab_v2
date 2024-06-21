import { getParticipantIdInTournamentServerOnly } from "$lib/api";
import { redirect } from "@sveltejs/kit";

export function load({ params, fetch, cookies }) {
    let participantId = getParticipantIdInTournamentServerOnly(cookies, params.tournament_id);

    if (participantId === null) {
        throw redirect(302, `/tournament/${params.tournament_id}/public`);
    }
    else {
        throw redirect(302, `/tournament/${params.tournament_id}/home/${participantId}`);
    }

    return {};
}