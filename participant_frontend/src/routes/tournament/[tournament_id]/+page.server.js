import { makeAuthenticatedRequestServerOnly } from "$lib/api";
import { redirect } from "@sveltejs/kit";

export async function load({ params, fetch, cookies }) {
    let participantId = null;
    try {
        let userInfo = await makeAuthenticatedRequestServerOnly(
            `api/user/tournament/${params.tournament_id}`,
            cookies,
            {}
        );    
        participantId = (await userInfo.json()).participant_id;
    }
    catch (e) {
    }

    if (participantId === null) {
        throw redirect(302, `/tournament/${params.tournament_id}/public`);
    }
    else {
        throw redirect(302, `/tournament/${params.tournament_id}/home/${participantId}`);
    }

    return {};
}