import { env } from '$env/dynamic/public'

/** @type {import('./$types').PageLoad} */
export async function load({ params, fetch }) {
    let result = await fetch(
        `${env.PUBLIC_API_URL}/api/register/${params.key}`
    );
    let data = await result.json();

    return {
        key: params.key,
        participant_name: data.participant_name,
        tournament_name: data.tournament_name,
    }
}