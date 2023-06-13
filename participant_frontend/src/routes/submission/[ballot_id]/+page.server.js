import { env } from '$env/dynamic/public'
import { make_authenticated_request } from '$lib/api';

/** @type {import('./$types').PageServerLoad} */
export async function load({ params, fetch, cookies }) {
    //const res = await fetch(`${env.PUBLIC_API_URL}/api/v1/ballot-submission/${params.ballot_id}`);
    let res = await make_authenticated_request(
        `api/submission/${params.ballot_id}`,
        cookies,
        {}
    )
    const ballot = await res.json();

    console.log(ballot);

    return {
        ballot: ballot.ballot,
        debate: {
            uuid: ballot.debate_id,
        }
    };
}