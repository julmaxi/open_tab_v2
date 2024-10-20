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
    let { declared_clashes, declared_institutions } = await clashReq.json();

    let data = {
        declared_clashes,
        declared_institutions,
        isEditing: false
    };

    if (url.searchParams.has('editClashes')) {
        let req = await makeAuthenticatedRequestServerOnly(
            `api/tournament/${params.tournament_id}/participants`,
            cookies,
            {}
        );
        let { adjudicators, teams } = await req.json();
        let targets = adjudicators.map(adjudicator => ({
            uuid: adjudicator.uuid,
            name: adjudicator.display_name,
            participant_role: "adjudicator",
        }));
        for (let team of teams) {
            for (let member of team.members) {
                targets.push({
                    uuid: member.uuid,
                    name: member.display_name,
                    participant_role: "speaker",
                });
            }
        }
        data.targets = targets;
        data.isEditing = "clashes";
    }
    else if (url.searchParams.has('editInstitutions')) {
        let req = await makeAuthenticatedRequestServerOnly(
            `api/tournament/${params.tournament_id}/institutions`,
            cookies,
            {}
        );
        let { institutions } = await req.json();
        data.targets = institutions.map(institution => ({
            uuid: institution.uuid,
            name: institution.name,
        }));
        data.isEditing = "institutions";
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

        let body = null;

        if (formData.get("clash_category") === "institutions") {
            body = JSON.stringify({
                added_institutions: Array.from(addedClashes),
                removed_institutions: Array.from(removedClashes)
            })
        } else {
            body = JSON.stringify({
                added_clashes: Array.from(addedClashes),
                removed_clashes: Array.from(removedClashes)
            })
        }

        await makeAuthenticatedRequestServerOnly(
            `api/participant/${params.participant_id}/clashes`,
            cookies,
            {
                method: 'POST',
                body,
                headers: {
                    'Content-Type': 'application/json'
                }
            }
        );            

        return {}
    }
}