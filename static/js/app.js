import {
    getFilter,
    getCache,
    getInstrumentation,
    getPassword,
    getAllowedDomains,
    postAllowedDomains,
    postUpdateFilter,
    deleteAllowedDomains,
} from './network.js';

const NSEC_PER_SEC = 1000000000;
let filter_created_at;

document.getElementById('login-button').addEventListener('click', main);
document.getElementById('reload').addEventListener('click', main);
document.getElementById('add-domain-name-button').addEventListener('click', addAllowedDomain);
document.getElementById('update-filter').addEventListener('click', updateFilter);

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

function showStatistics() {
    return getFilter().then(filter => {
        let entries = Object.entries(filter.statistics.data.data);
        entries.sort((a, b) => a[1][1] - b[1][1]);
        const table = document.getElementById('statistics');
        table.innerHTML = '';

        for(const entry of entries) {
            const row = table.insertRow(0);
            const cell0 = row.insertCell(0);
            const cell1 = row.insertCell(1);
            const cell2 = row.insertCell(2);
            cell0.innerHTML = entry[0];
            cell1.innerHTML = entry[1][0];
            cell2.innerText = new Date(entry[1][1].secs_since_epoch * 1000).toLocaleString();
        }

        document.getElementById('filter-size').innerText = `${filter.size} entries`;
        document.getElementById('filter-created-at').innerText = new Date(filter.created_at.secs_since_epoch * 1000);
        filter_created_at = filter.created_at.secs_since_epoch;

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

function removeAllowedDomain(domain) {
    deleteAllowedDomains(domain).then(main);
}

function addAllowedDomain() {
    const input = document.getElementById('add-domain-name').value;
    if (input === '') {
        return;
    }

    postAllowedDomains(input).then(main);
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
