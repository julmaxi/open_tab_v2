import { env } from "$env/dynamic/public";

export async function make_authenticated_request(
    url,
    cookies,
    options
) {
    let headers = {'Authorization': `Bearer ${cookies.get("token")}`};
    if (options.headers !== undefined) {
        headers = {...headers, ...options.headers};
        options.headers = undefined;
    }

    if (env.PUBLIC_API_URL === undefined) {
        throw Error("PUBLIC_API_URL is undefined");
    }

    const res = await fetch(
        `${env.PUBLIC_API_URL}/${url}`,
        {
            ...options,
            headers: new Headers(headers),
        }
    );
    if (res.status != 200) {
        console.error(Error(`Request to ${url} failed with status ${res.status}`))
    }
    return res   
}