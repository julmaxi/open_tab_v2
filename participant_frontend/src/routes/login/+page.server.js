import { makeAuthenticatedRequestServerOnly } from "$lib/api"
import { env } from "$env/dynamic/public"
import { error, fail, redirect } from "@sveltejs/kit";

export const actions = {
	create: async ({ cookies, request, fetch }) => {
        const formData = await request.formData();

        let url = "api/tokens";
        let userName = formData.get("user_name");
        let password = formData.get("password");
        let basicHeader = btoa(`mail#${userName}:${password}`);
        let headers = {'Authorization': `Basic ${basicHeader}`, 'Content-type': 'application/json'};
    
        if (env.PUBLIC_API_URL === undefined) {
            throw Error("PUBLIC_API_URL is undefined");
        }
    
        const res = await fetch(
            `${env.PUBLIC_API_URL}/${url}`,
            {
                method: 'POST',
                headers: new Headers(headers),
                body: JSON.stringify({}),
            }
        );
        if (res.status != 200) {
            let err = await res.text();
            console.log(Error(`Request to ${url} failed with status ${res.status}: ${err}`));
            if (res.status === 401) {
                return fail(400, {errors: ["Invalid username or password"]});
            }
            throw error(res.status, `Request to ${url} failed with status ${res.status}: ${err}`);
        }
        let response = await res.json();
        cookies.set("token", response.token);
        cookies.set("user_id", response.user_id);

        throw redirect(301, "/");

        return {};
    }
}