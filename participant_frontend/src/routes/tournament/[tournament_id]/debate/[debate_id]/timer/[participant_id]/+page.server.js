import { makeAuthenticatedRequestServerOnly } from "$lib/api";

//Set no ssr
export const ssr = false;

export async function load({ params, cookies }) {
    let res = await makeAuthenticatedRequestServerOnly(
        `api/debate/${params.debate_id}/timing`,
        cookies,
        {}
    )
    const timingInfo = (await res.json());

    return {
        timingInfo,
        participantId: params.participant_id
    };
}

export const actions = {
    resume: async ({request, params, cookies}) => {
        let formData = await request.formData();
        let speechRole = formData.get("speechRole");
        let speechPosition = formData.get("speechPosition");
        let isResponse = formData.get("isResponse") == "true";

        let end = new Date(formData.get("speechEnd"));
        let resumeTime = new Date(formData.get("resumeTime"));
        let previousPause = parseInt(formData.get("previousPause"));

        let pauseMilliseconds = resumeTime.getTime() - end.getTime() + previousPause;

        let pauseKey = isResponse ? "response_pause_milliseconds" : "pause_milliseconds";
        let endKey = isResponse ? "response_end" : "end";

        let body = {
            speech_role: speechRole,
            speech_position: parseInt(speechPosition),
        };

        body[pauseKey] = pauseMilliseconds;
        body[endKey] = null;

        let res = await makeAuthenticatedRequestServerOnly(
            `api/debate/${params.debate_id}/timing`,
            cookies,
            {
                method: "PATCH",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify(body),
            }
        );
    },
    setTime: async ({request, params, cookies}) => {
        let formData = await request.formData();

        let speechRole = formData.get("speechRole");
        let speechPosition = formData.get("speechPosition");
        let isResponse = formData.get("isResponse") == "true";

        let time = formData.get("time");
        let key = formData.get("key");

        if (isResponse) {
            key = "response_" + key;
        }

        let body = {
            speech_role: speechRole,
            speech_position: parseInt(speechPosition),
        };

        //Remove last letter
        time = time.substring(0, time.length - 1);
        body[key] = time

        let res = await makeAuthenticatedRequestServerOnly(
            `api/debate/${params.debate_id}/timing`,
            cookies,
            {
                method: "PATCH",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify(body),
            }
        );
    },
    reset: async ({request, params, cookies}) => {
        let formData = await request.formData();

        let speechRole = formData.get("speechRole");
        let speechPosition = formData.get("speechPosition");
        let isResponse = formData.get("isResponse") == "true";

        let body = {
            speech_role: speechRole,
            speech_position: parseInt(speechPosition),
        };

        if (isResponse) {
            body["response_start"] = null;
            body["response_end"] = null;
            body["response_pause_milliseconds"] = 0;
        }
        else {
            body["start"] = null;
            body["end"] = null;
            body["pause_milliseconds"] = 0;
        }

        let res = await makeAuthenticatedRequestServerOnly(
            `api/debate/${params.debate_id}/timing`,
            cookies,
            {
                method: "PATCH",
                headers: {
                    "Content-Type": "application/json",
                },
                body: JSON.stringify(body),
            }
        );
    }
}