import { env } from '$env/dynamic/public'

/** @type {import('./$types').PageLoad} */
export async function load({ params, fetch }) {
    return {
        key: params.key
    }
}