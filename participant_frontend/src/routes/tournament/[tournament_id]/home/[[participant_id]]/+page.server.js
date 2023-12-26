import { env } from '$env/dynamic/public'
import { getParticipantIdInTournament, makeAuthenticatedRequest } from '$lib/api';
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
        throw redirect(302, `/tournament/${params.tournament_id}/home/${getParticipantIdInTournament(cookies, params.tournament_id) }`);
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

export const actions = {
    releaseMotion: async ({request, params, cookies}) => {
        let formData = await request.formData();
        let debateId = formData.get("debateId");
        let releaseVal = formData.get("release") == "true";

        let res = await makeAuthenticatedRequest(
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