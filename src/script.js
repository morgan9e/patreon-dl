(async () => {

    const basePostURL = "https://www.patreon.com/api/posts?"

    const campaignID = Number(window.patreon.bootstrap.creator.data.id)
    const artistName = window.patreon.bootstrap.creator.data.attributes.name;

    console.log("Sending Patreon information to patreon-dl...")

    await fetch("http://localhost:8080/user", {
        method: 'POST',
        headers: {"content-type": "application/json"},
        body: JSON.stringify({
            id: campaignID,
            name: window.patreon.bootstrap.creator.data.attributes.name
        })
    });

    const initialQueryParams = new URLSearchParams({
        "include": "images,media",
        "fields[post]": "post_metadata",
        "fields[media]": "id,image_urls,download_url,metadata,file_name",
        "filter[campaign_id]": campaignID,
        "filter[contains_exclusive_posts]": true,
        "sort": "-published_at",
        "json-api-version": "1.0"
    })

    let downloads = [];
    let posts = [];

    const initalPostRequest = await fetch(basePostURL + initialQueryParams.toString())
    const parsedInital = await initalPostRequest.json()

    let initialLength = 0;
    if ("included" in parsedInital) {
        initialLength = parsedInital.included.length
    }

    posts = posts.concat(parsedInital.data);
    
    for (let i = 0; i < initialLength; i++) {
        if(parsedInital.included[i].attributes.file_name === null) {
            continue
        }

        const originalFilename = parsedInital.included[i].attributes.file_name.split(".")
        const fileExtension = originalFilename.pop()
        const newFilename = `${originalFilename.join(".")}-${parsedInital.included[i].id}.${fileExtension}`

        downloads.push({file: parsedInital.included[i].attributes.file_name, id: parsedInital.included[i].id, url: parsedInital.included[i].attributes.download_url});
    }

    console.log(`Collected ${downloads.length} posts...`)

    let nextURL = ""
    if ("links" in parsedInital) {
        nextURL = parsedInital.links.next;
    }

    while (nextURL !== "") {
        const recursivePostRequest = await fetch(nextURL)
        const parsedPosts = await recursivePostRequest.json()

        posts = posts.concat(parsedPosts.data);

        let includedLength = 0;
        if ("included" in parsedPosts) {
            includedLength = parsedPosts.included.length
        }

        for (let i = 0; i < includedLength; i++) {
            downloads.push({file: parsedPosts.included[i].attributes.file_name, id: parsedPosts.included[i].id, url: parsedPosts.included[i].attributes.download_url});
        }

        if ("links" in parsedPosts) {
            nextURL = parsedPosts.links.next;
        } else {
            nextURL = ""
        }
        console.log(`Collected ${downloads.length} posts...`)
    }

    let reverseLookup = {};
    
    for (let p of posts){
        for (let imgs of p.relationships.images.data) {
            reverseLookup[imgs.id] = p.id;
        }
        for (let imgs of p.relationships.media.data) {
            reverseLookup[imgs.id] = p.id;
        }
    }

    let request = [];

    function addFileDownload(filename, id, fileURL) {
        let newFilename = ""

        if(filename == null) {
            console.log("Skipping image you don't have access to...")
            return
        } else if(fileURL == null) {
            console.log("Skipping image you don't have access to...")
            return
        } else {
            let originalFilename = filename.split(".")
            if (reverseLookup[id] == undefined ) {
                console.error(`Cannot find post of image ${id} for ${url}`)
                return
            }
            
            let postId = reverseLookup[id]
            let fileExtension = originalFilename.pop()
            newFilename = originalFilename.join(".")

            request.push({
                url:  fileURL,
                id:   id,
                post: postId,
                name: newFilename,
                ext:  fileExtension
            })
        }

    }

    for (let d of downloads) {
        addFileDownload(d.file, d.id, d.url)
    }

    console.log(`Sending ${request.length} image links to patreon-dl...`)

    await fetch("http://localhost:8080/download", {
        method: 'POST',
        headers: {"content-type": "application/json"},
        body: JSON.stringify({artist: artistName, data: request})
    });

    console.log("patreon-dl is starting download...")
})();