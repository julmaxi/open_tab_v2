import { env } from '$env/dynamic/public'
import { makeAuthenticatedRequest } from '$lib/api';

/** @type {import('./$types').PageServerLoad} */
export async function load({ params, fetch, cookies }) {
    //const res = await fetch(`${env.PUBLIC_API_URL}/api/v1/ballot-submission/${params.ballot_id}`);
    let res = await makeAuthenticatedRequest(
        `api/submission/${params.ballot_id}`,
        cookies,
        {}
    )
    const ballot = await res.json();

    return {
        ballot: ballot.ballot,
        debate: {
            uuid: ballot.debate_id,
        }
    };
}