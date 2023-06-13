import { env } from '$env/dynamic/public'
import { make_authenticated_request } from '$lib/api';

/** @type {import('./$types').PageServerLoad} */
export async function load({ params, fetch, cookies }) {
    /*const res = await fetch(
        `${env.PUBLIC_API_URL}/api/participant/${params.participant_id}`,
        {
            headers: new Headers({'Authorization': `Bearer ${cookies.get("token")}`}),
        }
    );*/
    let res = await make_authenticated_request(
        `api/participant/${params.participant_id}`,
        cookies,
        {}
    )
    const participant_info = await res.json();

    return {
        name: participant_info.name,
        rounds: participant_info.rounds,
    };
}