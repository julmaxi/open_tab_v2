import { env } from '$env/dynamic/public'
import { getParticipantIdInTournamentServerOnly, makeAuthenticatedRequestServerOnly } from '$lib/api';
import { redirect } from '@sveltejs/kit';

/** @type {import('./$types').PageServerLoad} */
export async function load({ params, fetch, cookies }) {
    /*const res = await fetch(
        `${env.PUBLIC_API_URL}/api/participant/${params.participant_id}`,
        {
            headers: new Headers({'Authorization': `Bearer ${cookies.get("token")}`}),
        }
    );*/
    let participantId = getParticipantIdInTournamentServerOnly(cookies, params.tournament_id);
    let res = await makeAuthenticatedRequestServerOnly(
        `api/participant/${participantId}/settings`,
        cookies,
        {}
    )
    const settings = await res.json();

    return {
        isAnonymous: settings.is_anonymous,
    };
}

/** @type {import('./$types').Actions} */
export const actions = {
    default: async ({request, params, cookies}) => {
        let participantId = getParticipantIdInTournamentServerOnly(cookies, params.tournament_id);
        const data = await request.formData();
        let res = await makeAuthenticatedRequestServerOnly(
            `api/participant/${participantId}/settings`,
            cookies,
            {
                method: "POST",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify({
                    is_anonymous: data.get("isAnonymous") == "t",
                }),
            }
        );
        return {
            status: 200,
        };
    }
}