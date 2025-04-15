import { makeAuthenticatedRequestServerOnly } from '$lib/api';

/** @type {import('./$types').PageServerLoad} */
export async function load({ params, cookies }) {
    let res = await makeAuthenticatedRequestServerOnly(
        `api/user/${params.user_id}/stats`,
        cookies,
        {}
    )
    const statistics = (await res.json());

    return {
        statistics: statistics
    };
}