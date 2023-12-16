import { env } from '$env/dynamic/public'
import { makeAuthenticatedRequest } from '$lib/api';
import { redirect } from '@sveltejs/kit';

/** @type {import('./$types').PageServerLoad} */
export async function load({ params, fetch, cookies }) {
    let res = await makeAuthenticatedRequest(
        `api/participant/${params.participant_id}/feedback`,
        cookies,
        {}
    )
    const feedbackSummary = await res.json();

    return {
        individualValues: feedbackSummary.individual_values,
        summaryValues: feedbackSummary.summary_values,
    };
}