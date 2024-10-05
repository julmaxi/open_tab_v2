import { env } from '$env/dynamic/public'
import { makeAuthenticatedRequestServerOnly } from '$lib/api';
import { redirect } from '@sveltejs/kit';

/** @type {import('./$types').PageLoad} */
export async function load({ params, fetch, cookies }) {    
    if (params.participant_id === undefined) {
        let userInfo = await makeAuthenticatedRequestServerOnly(
            `api/user/tournament/${params.tournament_id}`,
            cookies,
            {}
        );
        let participantId = (await userInfo.json()).participant_id;            
        throw redirect(302, `/tournament/${params.tournament_id}/home/${participantId}`);
    }  
    let res = null;
    try {
        res = await makeAuthenticatedRequestServerOnly(
            `api/participant/${params.participant_id}`,
            cookies,
            {}
        );    
    }
    catch (e) {
        if (e.status === 401) {
            throw redirect(302, `/tournament/${params.tournament_id}/public`);
        }
        throw e;
    }

    const participant_info = await res.json();

    return {
        name: participant_info.name,
        participantId: params.participant_id,
        rounds: participant_info.rounds,
        feedback_submissions: participant_info.feedback_submissions,
        tournamentId: params.tournament_id,
        role: participant_info.role,
        expectedReload: participant_info.expected_reload
    };
}

export const actions = {
    releaseMotion: async ({request, params, cookies}) => {
        let formData = await request.formData();
        let debateId = formData.get("debateId");
        let releaseVal = formData.get("release") == "true";

        let res = await makeAuthenticatedRequestServerOnly(
            `api/debate/${debateId}/state`,
            cookies,
            {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify({
                    release: releaseVal,
                    state: "NonAlignedMotionRelease"
                }),
            }
        );

        return {isMotionReleasedToNonAligned: releaseVal};

        //Prevents form resubmission
        //throw redirect(302, `/tournament/${params.tournament_id}/home/${params.participant_id}`);
    },
}