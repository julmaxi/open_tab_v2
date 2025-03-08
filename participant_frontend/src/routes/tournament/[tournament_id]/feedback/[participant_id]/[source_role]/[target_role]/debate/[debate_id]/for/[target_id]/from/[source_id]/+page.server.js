import { makeAuthenticatedRequestServerOnly, makeAuthenticatedRequestServerOnlyNoThrow } from '$lib/api';
import { fail, redirect } from '@sveltejs/kit';

/** @type {import('./$types').PageServerLoad} */
export async function load({ params, cookies }) {
    let res = await makeAuthenticatedRequestServerOnly(
        `api/feedback/${params.source_role}/${params.target_role}/debate/${params.debate_id}/for/${params.target_id}/from/${params.source_id}`,
        cookies,
        {}
    )
    const feedback_form = (await res.json());

    return {
        feedback_form: feedback_form
    };
}

/** @type {import('./$types').Actions} */
export const actions = {
    default: async ({request, params, cookies}) => {
        let formData = await request.formData();

        let jsonForm = {};
        
        for (let [key, val] of formData.entries()) {
            if (key.endsWith("_type")) {
                continue;
            }
            let type = formData.get(`${key}_type`);
            if (type === "int") {
                if (val != "") {
                    jsonForm[key] = {val: parseInt(val.toString())};
                }
            }
            else if (type === "bool") {
                if (val === "yes") {
                    jsonForm[key] = {val: true};
                }
                else if (val === "no") {
                    jsonForm[key] = {val: false};
                }
            }
            else if (type === "string") {
                jsonForm[key] = {val: val.toString()};
            }
            else {
                throw new Error(`Unknown type ${type}`);
            }
        }

        let submitUrl = `api/feedback/${params.source_role}/${params.target_role}/debate/${params.debate_id}/for/${params.target_id}/from/${params.source_id}`;

        let res = await makeAuthenticatedRequestServerOnlyNoThrow(submitUrl, cookies, {
            body: JSON.stringify({"answers": jsonForm}),
            method: "POST",
            headers: {"Content-Type": "application/json"},
        });
        
        switch (res.status) {
            case 200:
                throw redirect(302, `/tournament/${params.tournament_id}/home/`);
            case 400:
                let values = await res.json();
                return fail(400, {
                    ...values
                });
            default:
                throw Error(`Request to ${submitUrl} failed with status ${res.status}: ${await res.text()}`);
        }
    }
}