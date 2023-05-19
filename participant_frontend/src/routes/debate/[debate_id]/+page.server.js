import { env } from '$env/dynamic/public'
import { redirect } from '@sveltejs/kit';


function getPathAndCreateIntermediary(obj, path, permissibleFields) {
    if (path.length == 0) {
        return obj;
    }
    if (!isFinite(path[0]) && !permissibleFields.includes(path[0])) {
        throw new Error(`Field ${path[0]} not permitted`);
    }
    let val = obj[path[0]];
    if (val === undefined) {
        val = {};
        obj[path[0]] = val;
    }
    return getPathAndCreateIntermediary(val, path.slice(1), permissibleFields);
}


function parseFormToObject(formData, permissibleFields, maxDepth) {
    let out = {};
    for (let [key, val] of formData.entries()) {        
        let parts = key.split('.');
        if (parts.length > maxDepth) {
            throw new Error("Form data too deep");
        }
        let obj = getPathAndCreateIntermediary(out, parts.slice(0, parts.length - 1), permissibleFields)
        let finalField = parts[parts.length - 1];
        if (!isFinite(finalField) && !permissibleFields.includes(finalField)) {
            throw new Error(`Field ${parts[finalField]} not permitted`);
        }
        obj[finalField] = val;
    }
    return out;
}

function parseScores(scoresList, adjudicators) {
    let scores = {};
    for (let i = 0; i < adjudicators.length; i++) {
        let adj = adjudicators[i];
        let score = scoresList[i];

        if (score !== undefined && score !== null && score !== "") {
            scores[adj] = {
                "total": parseInt(score),
                "type": "Aggregate"
            }
        }
    }
    return scores;
}


/** @type {import('./$types').Actions} */
export const actions = {
    default: async (event) => {
        let formData = await event.request.formData();

        let ballot = {
            uuid: "00000000-0000-0000-0000-000000000000",
            government: {
                team: null,
                scores: null
            },
            opposition: {
                team: null,
                scores: null
            },
            adjudicators: [],
            speeches: [],

            president: formData.get("president") || null,
        };

        let values = parseFormToObject(
            formData,
            ["team", "scores", "speeches", "label", "government", "opposition", "adjudicators", "role", "position", "president"],
            5
        );

        ballot.government.team = values.government.label;
        ballot.opposition.team = values.opposition.label;
        let adjs = Object.entries(values.adjudicators).map(
            ([key, val]) => {
                return {
                    id: val,
                    index: parseInt(key)
                };
            }
        );
        adjs.sort((a, b) => a.index - b.index);
        ballot.adjudicators = adjs.map(adj => adj.id);

        ballot.government.scores = parseScores(values.government.scores, ballot.adjudicators);
        ballot.opposition.scores = parseScores(values.opposition.scores, ballot.adjudicators);

        let speeches = Object.entries(values.speeches).map(
            ([key, val]) => {
                return {
                    index: parseInt(key),
                    position: parseInt(val.position),
                    role: val.role,
                    speaker: val.label,
                    scores: parseScores(val.scores, ballot.adjudicators)
                };
            }
        );
        speeches.sort((a, b) => a.index - b.index);
        speeches = speeches.map((speech) => {
            delete speech.index;
            return speech;
        });

        ballot.speeches = speeches;

        let res = await fetch(`${env.PUBLIC_API_URL}/api/v1/debate/${event.params.debate_id}/ballots`, {
            method: "POST",
            body: JSON.stringify(ballot),
        });

        throw redirect(302, `/submission/${(await res.json()).debate_ballot_uuid}`);
    }
};