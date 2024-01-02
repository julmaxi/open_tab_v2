/** @type {import('@sveltejs/kit').HandleFetch} */
export async function handleFetch({ event, request, fetch }) {
	if (request.headers.get('Authorization') == "missing") {
        console.log(event.request.headers.get('cookie'));
		request.headers.set('Authorization', event.request.headers.get('cookie'));
	}

	return fetch(request);
}


export function handle({ event, resolve }) {
    for (let cookie of event.cookies.getAll()) {
        console.log(cookie);
    }
    //event.params.participant_id = event.cookies.get('my-cookie');
    return resolve(event);
  }
  