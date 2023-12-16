import { env } from '$env/dynamic/public'
import { redirect } from '@sveltejs/kit';
import { getParticipantCookieNameInTournament } from '$lib/api';

/** @type {import('./$types').Actions} */
export const actions = {
    default: async (event) => {
        let formData = await event.request.formData();

        let res = await fetch(`${env.PUBLIC_API_URL}/api/register`, {
            method: "POST",
            body: JSON.stringify({"secret": formData.get("key")}),
            headers: new Headers({'content-type': 'application/json'}),
        });

        if (res.status == 200) {
            let json = await res.json();

            let cookieName = getParticipantCookieNameInTournament(json.tournament_id);

            event.cookies.set("token", json.token, {sameSite: true, path: "/"});
            event.cookies.set(cookieName, json.participant_id, {sameSite: true, path: "/"});
            throw redirect(302, `/tournament/${json.tournament_id}/home/${json.participant_id}`);
        }

        //throw redirect(302, `/submission/${(await res.json()).debate_ballot_uuid}`);
    }
};