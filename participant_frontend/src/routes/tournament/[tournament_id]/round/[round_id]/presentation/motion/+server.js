import { makeAuthenticatedRequest } from '$lib/api';
import { error } from '@sveltejs/kit';

/** @type {import('./$types').RequestHandler} */
export async function POST({ params, cookies }) {
    let res = await makeAuthenticatedRequest(
        `api/draw/${params.round_id}/release-motion`,
        cookies,
        {
			method: 'POST',
		}
    )
    const info = await res.json();
	return new Response(JSON.stringify(info));
}