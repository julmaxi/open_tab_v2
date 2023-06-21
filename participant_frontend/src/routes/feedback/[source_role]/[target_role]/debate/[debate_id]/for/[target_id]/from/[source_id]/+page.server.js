import { make_authenticated_request } from '$lib/api';

/** @type {import('./$types').PageServerLoad} */
export async function load({ params, cookies }) {
    let res = await make_authenticated_request(
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
                jsonForm[key] = {val: parseInt(val.toString())};
            }
            else if (type === "bool") {
                jsonForm[key] = {val: val === "yes"};
            }
            else if (type === "string") {
                jsonForm[key] = {val: val.toString()};
            }
            else {
                throw new Error(`Unknown type ${type}`);
            }
        }

        let submitUrl = `api/feedback/${params.source_role}/${params.target_role}/debate/${params.debate_id}/for/${params.target_id}/from/${params.source_id}`;

        let res = await make_authenticated_request(submitUrl, cookies, {
            body: JSON.stringify({"answers": jsonForm}),
            method: "POST",
            headers: {"Content-Type": "application/json"},
        });
        console.log(res.status);
        console.log(await res.text());
    },
}