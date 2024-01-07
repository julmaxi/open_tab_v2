import { makeAuthenticatedRequestServerOnly } from "$lib/api";


export async function POST({ cookies, params, request, ...obj }) {
    let debateId = params.debate_id;
    let json = await request.text();
    console.log(">>", json, params, "<<");
    let response = await makeAuthenticatedRequestServerOnly(
        `api/debate/${debateId}/timing/notify`,
        cookies,
        {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: json
        }
    );
    
    return new Response("");
}