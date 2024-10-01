import { makeAuthenticatedRequestServerOnly } from "$lib/api";


export async function POST({ cookies }) {
    let response = await makeAuthenticatedRequestServerOnly(
        "api/tokens",
        cookies,
        {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify({}),
        }
    );
    
    let tokenInfo = (await response.json());

    return new Response(JSON.stringify({token: tokenInfo.token, expires: tokenInfo.expires}));
}