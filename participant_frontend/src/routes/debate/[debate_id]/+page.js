import { env } from '$env/dynamic/public'

/** @type {import('./$types').PageLoad} */
export async function load({ params, fetch }) {
    const res = await fetch(`${env.PUBLIC_API_URL}/api/v1/debate/${params.debate_id}`);
    const ballot = await res.json();

    return {
        ballot: ballot.ballot
    };
}