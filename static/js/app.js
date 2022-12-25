import {
    getFilter,
    getCache,
    getInstrumentation,
    getPassword,
    getAllowedDomains,
    getAutoUpdateFilter,
    getOverrides,
    postAutoUpdateFilter,
    postAllowedDomains,
    postUpdateFilter,
    postOverride,
    deleteAllowedDomains,
    deleteOverride,
} from './network.js';

const NSEC_PER_SEC = 1000000000;
let filter_created_at;
let filtered = [];
let filteredOrder = ['updated_at', 'desc'];
let latestFilter;

document.getElementById('login-button').addEventListener('click', main);
document.getElementById('reload').addEventListener('click', main);
document.getElementById('add-domain-name-button').addEventListener('click', addAllowedDomain);
document.getElementById('overrides-add').addEventListener('click', addOverride);
document.getElementById('update-filter').addEventListener('click', updateFilter);
document.getElementById('auto-update-button').addEventListener('click', updateAutoUpdateFilter);
document.getElementById('auto-update-checkbox').addEventListener('change', toggleAutoUpdateTextbox);

document.getElementById('count').addEventListener('click', () => {
    filteredOrder[0] = 'count';
    filteredOrder[1] = filteredOrder[1] === 'asc' ? 'desc' : 'asc';
    displayFiltered(filtered);
});
document.getElementById('updated-at').addEventListener('click', () => {
    filteredOrder[0] = 'updated_at';
    filteredOrder[1] = filteredOrder[1] === 'asc' ? 'desc' : 'asc';
    displayFiltered(filtered);
});

document.getElementById('login-password').addEventListener('keyup', function(event) {
    if (event.keyCode === 13) {
        event.preventDefault();
        document.getElementById("login-button").click();
    }
});

document.getElementById('add-domain-name').addEventListener('keyup', function(event) {
    if (event.keyCode === 13) {
        event.preventDefault();
        addAllowedDomain();
    }
});

if(getPassword()) {
    main();
}

function main() {
    showStatistics()
        .then(showMain)
        .then(showCache)
        .then(showInstrumentation)
        .then(showAllowedDomains)
        .then(showOverrides)
        .then(showAutoUpdate)
        .catch(e => {
            alert(e);
            document.getElementById('login-container').style.display = 'block';
            document.getElementById('main-container').style.display = 'none';
        });
}

function showMain() {
    document.getElementById('add-domain-name').value = '';
    const password = document.getElementById('login-password').value;
    if(password) {
        sessionStorage.setItem("password", password);
    }
    document.getElementById('login-container').style.display = 'none';
    document.getElementById('main-container').style.display = 'block';
    return Promise.resolve();
}

function displayFiltered(entries) {
    document.getElementById('count').innerText = 'Count ';
    document.getElementById('updated-at').innerText = 'Updated at ';

    if (filteredOrder[0] === 'updated_at' && filteredOrder[1] === 'desc') {
        filtered.sort((a, b) => new Date(a[1][1].secs_since_epoch * 1000) - new Date(b[1][1].secs_since_epoch * 1000));
    } else if (filteredOrder[0] === 'updated_at' && filteredOrder[1] === 'asc') {
        filtered.sort((a, b) => new Date(b[1][1].secs_since_epoch * 1000) - new Date(a[1][1].secs_since_epoch * 1000));
    } else if (filteredOrder[0] === 'count' && filteredOrder[1] === 'desc') {
        filtered.sort((a, b) => a[1][0] - b[1][0]);
    } else {
        filtered.sort((a, b) => b[1][0] - a[1][0]);
    }

    const image = document.createElement('img');
    image.src = filteredOrder[1] === 'desc' ? 'icons/caret-down-solid.svg' : 'icons/caret-up-solid.svg';
    image.className = 'remove-icon';
    const element = filteredOrder[0] === 'updated_at' ? document.getElementById('updated-at') :
        document.getElementById('count');
    element.appendChild(image);

    const table = document.getElementById('statistics');
    table.innerHTML = '';

    for(const entry of entries) {
        const row = table.insertRow(0);
        const cell0 = row.insertCell(0);
        const cell1 = row.insertCell(1);
        const cell2 = row.insertCell(2);
        const cell3 = row.insertCell(3);
        cell0.innerHTML = entry[0];
        cell1.innerHTML = entry[1][0];
        cell2.innerText = new Date(entry[1][1].secs_since_epoch * 1000).toLocaleString();

        const image = document.createElement('img');
        cell3.addEventListener('click', () => allowDomain(entry[0]))
        cell3.innerText = "+";
        cell3.className = 'allow-domain-column';
    }

    document.getElementById('filter-size').innerText = `${latestFilter.size} entries`;
    document.getElementById('filter-created-at').innerText = new Date(latestFilter.created_at.secs_since_epoch * 1000);
    filter_created_at = latestFilter.created_at.secs_since_epoch;
}

async function showAutoUpdate() {
    return getAutoUpdateFilter().then(autoUpdate => {
        const checkbox = document.getElementById('auto-update-checkbox');
        const text = document.getElementById('auto-update-value');
        if(!autoUpdate) {
            checkbox.checked = false;
        } else {
            checkbox.checked = true;
            text.value = autoUpdate;
        }

        toggleAutoUpdateTextbox();
    })
}

function showStatistics() {
    return getFilter().then(filter => {
        latestFilter = filter;
        filtered = Object.entries(filter.statistics.data.data);
        displayFiltered(filtered);

        return Promise.resolve();
    });
}

function showCache() {
    return getCache().then(cache => {
        const table = document.getElementById('cache');
        table.innerHTML = ""

        cache.data.data.sort((a, b) => a.valid_until - b.valid_until);

        for(const entry of cache.data.data) {
            const row = table.insertRow(0);
            const cell0 = row.insertCell(0);
            const cell1 = row.insertCell(1);
            cell0.innerHTML = entry.message.name;
            const date = new Date(0);
            date.setUTCSeconds(entry.valid_until);
            cell1.innerHTML = date.toLocaleString();
        }

        const count = document.getElementById('cache-count');
        count.textContent = cache.data.data.length;

        return Promise.resolve();
    });
}

function showInstrumentation() {
    return getInstrumentation().then(instrumentation => {
        const table = document.getElementById('instrumentation');
        table.innerHTML = ""

        let resolvers = instrumentation.data.container.reduce((acc, entry) => {
            if(entry.resolver === null) {
                return acc;
            }
            if(!(entry.resolver in acc)) {
                acc[entry.resolver] = [];
            }
            const diffSecs = entry.request_received.secs_since_epoch - entry.request_sent.secs_since_epoch;
            const diff = (diffSecs * NSEC_PER_SEC) + entry.request_received.nanos_since_epoch - entry.request_sent.nanos_since_epoch;
            acc[entry.resolver].push(diff);
            return acc;
        }, {});

        let entries = Object.entries(resolvers);
        entries = entries.map((entry) => {
            const sum = entry[1].reduce((acc, item) => acc + item, 0)
            return { resolver: entry[0], average: sum / entry[1].length / (Math.pow(10, 6)), count: entry[1].length };
        });

        for(const entry of entries) {
            const row = table.insertRow(0);
            const cell0 = row.insertCell(0);
            const cell1 = row.insertCell(1);
            const cell2 = row.insertCell(2);
            cell0.innerHTML = entry.resolver;
            cell1.innerHTML = entry.average;
            cell2.innerHTML = entry.count;
        }

        return Promise.resolve();
    });
}

function showAllowedDomains() {
    return getAllowedDomains().then(domains => {
        const table = document.getElementById('allowed-domains');
        table.innerHTML = "";
        for(const domain of domains) {
            const row = table.insertRow(0);
            const cell0 = row.insertCell(0);
            const cell1 = row.insertCell(1);
            const image = document.createElement('img');
            image.src = 'icons/trash-solid.svg';
            image.alt = 'Trash icon';
            image.className = 'remove-icon';
            image.addEventListener('click', () => removeAllowedDomain(domain))
            cell0.innerHTML = domain;
            cell1.appendChild(image);
            cell1.className = 'remove-allowed-domains-column';
        }

        return Promise.resolve();
    });
}

function showOverrides() {
    return getOverrides().then(overrides => {
        const table = document.getElementById('overrides-domains');
        table.innerHTML = "";
        for(const [domain, address] of Object.entries(overrides)) {
            const row = table.insertRow(0);
            const cell0 = row.insertCell(0);
            const cell1 = row.insertCell(1);
            const cell2 = row.insertCell(2);
            const image = document.createElement('img');
            image.src = 'icons/trash-solid.svg';
            image.alt = 'Trash icon';
            image.className = 'remove-icon';
            image.addEventListener('click', () => removeOverride(domain))
            cell0.innerHTML = domain;
            cell1.innerHTML = address.join(".");
            cell2.appendChild(image);
            cell2.className = 'remove-allowed-domains-column';
        }

        return Promise.resolve();
    });
}

function removeAllowedDomain(domain) {
    deleteAllowedDomains(domain).then(main);
}

function allowDomain(domain) {
    postAllowedDomains(domain).then(main);
}

function removeOverride(domain) {
    deleteOverride(domain).then(main);
}

function addAllowedDomain() {
    const input = document.getElementById('add-domain-name').value;
    if (input === '') {
        return;
    }

    postAllowedDomains(input).then(main);
}

function addOverride() {
    const name = document.getElementById('overrides-name').value;
    const address = document.getElementById('overrides-address').value;
    if (name === '' || address === '') {
        return;
    }

    postOverride(name, address).then(main);
}

function sleep(m) {
    return new Promise(r => setTimeout(r, m));
}

function updateFilter() {
    const button = document.getElementById('update-filter');
    button.disabled = true;
    let seen_created_at = filter_created_at;
    const checkFilter = () => {
        return showStatistics().then(async () => {
            if (filter_created_at === seen_created_at) {
                await sleep(1000);
                return checkFilter();
            } else {
                button.disabled = false;
            }
        });
    };
    return postUpdateFilter().then(showStatistics).then(checkFilter);
}

function updateAutoUpdateFilter() {
    let value = null;
    if(document.getElementById('auto-update-checkbox').checked) {
        value = parseInt(document.getElementById('auto-update-value').value);
    }
    const button = document.getElementById('auto-update-button');
    button.disabled = true;
    return postAutoUpdateFilter(value)
        .then(result => {
            console.log(result);
            button.disabled = false;
        })
}

function toggleAutoUpdateTextbox() {
    const container = document.getElementById('auto-update-div');
    if (document.getElementById('auto-update-checkbox').checked) {
        container.style = '';
    } else {
        container.style = 'display: none';
    }
}
