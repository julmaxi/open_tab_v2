import { env } from '$env/dynamic/public'
import { redirect } from '@sveltejs/kit';
import { makeAuthenticatedRequestServerOnly } from '$lib/api';

/** @type {import('./$types').Actions} */
export const actions = {
    register: async (event) => {
        let formData = await event.request.formData();

        let res = await fetch(`${env.PUBLIC_API_URL}/api/register`, {
            method: "POST",
            body: JSON.stringify({"secret": formData.get("key")}),
            headers: new Headers({'content-type': 'application/json'}),
        });

        console.log(res.status);
        if (res.status == 200) {
            let json = await res.json();

            event.cookies.set("token", json.token, {sameSite: true, path: "/", maxAge: 60 * 60 * 24 * 7 * 4});
            throw redirect(302, `/tournament/${json.tournament_id}/home/${json.participant_id}`);
        }
    },
    registerAsUser: async (event) => {
        let formData = await event.request.formData();

        let res = await makeAuthenticatedRequestServerOnly(
            "api/register",
            event.cookies,
            {
                method: "POST",
                body: JSON.stringify({"secret": formData.get("key"), "link_current_user": true}),
                headers: {"Content-Type": "application/json"},
            }
        )

        if (res.status == 200) {
            let json = await res.json();
            throw redirect(302, `/tournament/${json.tournament_id}/home/${json.participant_id}`);
        }
    }
};

/** @type {import('./$types').PageServerLoad} */
export async function load({ params, fetch, cookies }) {
    let result = null;
    
    if (cookies.get("token")) {
        result = await makeAuthenticatedRequestServerOnly(
            `api/register/${params.key}`,
            cookies,
            {}
        );
    }
    else {
        result = await fetch(
            `${env.PUBLIC_API_URL}/api/register/${params.key}`
        );
    }
    let data = await result.json();

    return {
        key: params.key,
        participant_name: data.participant_name,
        tournament_name: data.tournament_name,
        canClaimAsUser: data.user_can_claim_participant
    }
}