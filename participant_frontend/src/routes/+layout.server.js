import { invalidateAll } from '$app/navigation';
import { env } from '$env/dynamic/public'
import { makeAuthenticatedRequestServerOnly, makeRequest } from '$lib/api';
import { redirect } from '@sveltejs/kit';


/** @type {import('./$types').PageServerLoad} */
export async function load({ params, fetch, cookies, url }) {
    let path = url.pathname;
    let layoutInfo = {};
    let userInfo = null;
    try {
        let userInfoRequest = await makeAuthenticatedRequestServerOnly(
            "api/user",
            cookies,
            {}
        );
        userInfo = await userInfoRequest.json().catch(() => null);    
    }
    catch (e) {
        
    }

    if (userInfo === null) {
        layoutInfo["isAuthenticated"] = false;
    }
    else {
        layoutInfo["isAuthenticated"] = true;
        layoutInfo["userIdentifier"] = userInfo.identifier;
    }

    if (path.startsWith("/tournament/")) {
        layoutInfo = {
            ...layoutInfo,
            ...await loadTournamentInfo({ params, fetch, cookies, url }),
        };
    }
    else {
        layoutInfo = {
            ...layoutInfo,
            pageTitle: "OpenTab",
            titleLink: "/",
            additionalLinks: []
        };
    }

    // Add hideNavbar field based on the URL
    layoutInfo["hideNavbar"] = !!path.match("/tournament/[a-z0-9-]+/admin/round/[a-z0-9-]+/presentation");

    return layoutInfo;
}

async function loadTournamentInfo({ params, fetch, cookies, url }) {
    let participantId = null;
    if (params.participant_id !== undefined) {
        participantId = params.participant_id;
    }
    else {
        try {
            let userInfo = await makeAuthenticatedRequestServerOnly(
                `api/user/tournament/${params.tournament_id}`,
                cookies,
                {}
            );    
            participantId = (await userInfo.json()).participant_id;
        }
        catch (e) {
        }    
    }
    let additionalLinks = [];

    let tournamentName = "";
    let participantInfo = null;
    if (participantId !== null) {
        try {
            let res = await makeAuthenticatedRequestServerOnly(
                `api/participant/${participantId}/info`,
                cookies,
                {}
            );
            participantInfo = await res.json();    
        }
        catch (e) {
        }
    }

    if (participantInfo !== null) {
        additionalLinks.push(
            {
                name: "Overview",
                url: `/tournament/${params.tournament_id}/home`,
            }
        )

        additionalLinks.push(
            {
                name: "Tab",
                url: `/tournament/${params.tournament_id}/tab`,
            }
        )

        if (participantInfo.can_edit_clashes) {
            additionalLinks.push(
                {
                    name: "Clashes",
                    url: `/tournament/${params.tournament_id}/home/${participantId}/clashes`,
                }
            )
        }

        additionalLinks.push(
            {
                name: "Settings",
                url: `/tournament/${params.tournament_id}/home/${participantId}/settings`,
            }
        )

        additionalLinks.push(
            {
                name: "Participants",
                url: `/tournament/${params.tournament_id}/participants`,
            }
        )
    
        if (participantInfo.role.type == "Adjudicator") {
            additionalLinks.push({
                name: "Feedback",
                url: `/tournament/${params.tournament_id}/home/${participantId}/feedback`,
            });
        }

        tournamentName = participantInfo.tournament_name;
    }
    else {
        let public_info = await makeAuthenticatedRequestServerOnly(
            `api/tournament/${params.tournament_id}/public`,
            cookies,
            {}
        )
        public_info = await public_info.json();

        additionalLinks.push(
            {
                name: "Overview",
                url: `/tournament/${params.tournament_id}/public`,
            }
        );

        if (public_info.show_tab) {
            additionalLinks.push(
                {
                    name: "Tab",
                    url: `/tournament/${params.tournament_id}/tab`,
                }
            );
        }

        if (public_info.show_participants) {
            additionalLinks.push(
                {
                    name: "Participants",
                    url: `/tournament/${params.tournament_id}/participants`,
                }
            );
        }

        tournamentName = public_info.tournament_name;
    }

    return {
        additionalLinks,
        tournamentId: params.tournament_id,
        titleLink: `/tournament/${params.tournament_id}`,
        pageTitle: tournamentName,
    };
}