import { makeAuthenticatedRequestServerOnly } from "$lib/api"
import { env } from "$env/dynamic/public"
import { error, fail, redirect } from "@sveltejs/kit";
import { invalidateAll } from "$app/navigation";


/** @type {import('./$types').RequestHandler} */
export async function POST({ params, cookies }) {
    try {
        await makeAuthenticatedRequestServerOnly(
            "api/token",
            cookies,
            { method: "DELETE" }
        );
    } catch (e) {
        throw redirect(301, "/");
    }
    cookies.delete("token");
    cookies.delete("user_id");

    for (let item of cookies.getAll()) {
        if (item.name.startsWith("participant_id:")) {
            cookies.delete(item.name);
        }
        cookies.delete(item.name);
    }

    throw redirect(301, "/");
}