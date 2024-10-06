import { env } from '$env/dynamic/public'
import { makeAuthenticatedRequestServerOnly } from '$lib/api';
import { redirect } from '@sveltejs/kit';

/** @type {import('./$types').PageServerLoad} */
export async function load({ params, fetch, cookies, url }) {    
    let clashReq = await makeAuthenticatedRequestServerOnly(
        `api/participant/${params.participant_id}/clashes`,
        cookies,
        {}
    );
    let { declared_clashes } = await clashReq.json();

    let data = {
        declared_clashes,
        isEditing: false
    };

    if (url.searchParams.has('edit')) {
        console.log('edit');
        let req = await makeAuthenticatedRequestServerOnly(
            `api/tournament/${params.tournament_id}/participants`,
            cookies,
            {}
        );
        let { adjudicators, teams } = await req.json();
        let targets = adjudicators.map(adjudicator => ({
            uuid: adjudicator.uuid,
            participant_name: adjudicator.display_name,
            participant_role: "adjudicator",
        }));
        for (let team of teams) {
            for (let member of team.members) {
                targets.push({
                    uuid: member.uuid,
                    participant_name: member.display_name,
                    participant_role: "speaker",
                });
            }
        }
        data.targets = targets;
        data.isEditing = true;
    }
    return data;
}

export const actions = {
    updateClashes: async ({ cookies, params, request, fetch }) => {
        let formData = await request.formData();
        let selectedClashes = new Set(formData.getAll('clashes[]'));
        let previousClashes =  new Set(formData.getAll('previous_clashes[]'));

        let addedClashes = selectedClashes.difference(previousClashes);
        let removedClashes = previousClashes.difference(selectedClashes);

        await makeAuthenticatedRequestServerOnly(
            `api/participant/${params.participant_id}/clashes`,
            cookies,
            {
                method: 'POST',
                body: JSON.stringify({
                    added_clashes: Array.from(addedClashes),
                    removed_clashes: Array.from(removedClashes)
                }),
                headers: {
                    'Content-Type': 'application/json'
                }
            }
        );            

        return {}
    }
}