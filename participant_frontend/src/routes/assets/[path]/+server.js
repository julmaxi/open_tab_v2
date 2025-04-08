import { env } from "$env/dynamic/public";
import { makeAuthenticatedRequestServerOnly, makeRequest } from "$lib/api";


export async function GET({ cookies, params, fetch }) {
    if (env.PUBLIC_API_URL === undefined) {
        throw Error("PUBLIC_API_URL is undefined");
    }

    let response = await fetch(
        env.PUBLIC_API_URL + "/assets/" + params.path,
    );
    
    return new Response(response.body, {
        status: response.status,
        headers: {
            "Content-Type": response.headers.get("Content-Type"),
        }
    });
}