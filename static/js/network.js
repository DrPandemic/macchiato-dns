const API_PATH = '/api/1'

export function getPassword() {
    return sessionStorage.getItem('password') || document.getElementById('password').value;
}

function doCall(name, verb = 'GET', payload = null) {
    let options = {
        method: verb,
        headers: {
            authorization: `Bearer ${getPassword()}`,
            'Content-Type': 'application/json'
        },
    };
    if (payload !== null) {
        options['body'] = JSON.stringify(payload);
    }

    return fetch(`${API_PATH}/${name}`, options)
        .then(response => {
        if(response.status === 200) {
            return response.json();
        } else if(response.status === 401) {
            sessionStorage.removeItem('password')
        }
        return Promise.reject(new Error(response.statusText));
    });
}

export function getFilter() {
    return doCall('filter');
}

export function postUpdateFilter() {
    return doCall('update-filter', 'POST');
}

export function getCache() {
    return doCall('cache');
}

export function getInstrumentation() {
    return doCall('instrumentation');
}

export function getAllowedDomains() {
    return doCall('allowed-domains');
}

export function getAutoUpdateFilter() {
    return doCall('auto-update-filter');
}

export function postAllowedDomains(domain) {
    return doCall('allowed-domains', 'POST', { name: domain });
}

export function deleteAllowedDomains(domain) {
    return doCall('allowed-domains', 'DELETE', { name: domain });
}


export function postAutoUpdateFilter(autoUpdate) {
    return doCall('auto-update-filter', 'POST', { auto_update: autoUpdate });
}

export function getOverrides() {
    return doCall('overrides');
}

export function deleteOverride(override) {
    return doCall('overrides', 'DELETE', { name: override });
}

export function postOverride(name, address) {
    return doCall('overrides', 'POST', { name: name, address: address });
}

export function postDisable() {
    return doCall('disable', 'POST');
}
