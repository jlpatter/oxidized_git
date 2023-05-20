import {emit, listen} from "@tauri-apps/api/event";
import {open} from "@tauri-apps/api/dialog";
import {homeDir} from "@tauri-apps/api/path";

export class ExternalGitOps {
    constructor(mainJS) {
        this.mainJS = mainJS;
    }

    setListeners() {
        const self = this;
        listen("get-init", async function(ev) {
            await self.doInit();
        }).then();

        listen("get-open", async function(ev) {
            await self.doOpen();
        }).then();
    }

    setEvents() {
        const self = this;
        $('#wInitBtn').click(async function() {
            await self.doInit();
        });

        $('#wOpenBtn').click(async function() {
            await self.doOpen();
        });
    }

    async doInit() {
        const self = this,
            selected = await open({
                directory: true,
                multiple: false,
                defaultPath: await homeDir(),
            });
        if (selected !== null) {
            self.mainJS.addProcessCount();
            emit("init", selected).then();
        }
    }

    async doOpen() {
        const self = this,
            selected = await open({
                directory: true,
                multiple: false,
                defaultPath: await homeDir(),
            });
        if (selected !== null) {
            self.mainJS.addProcessCount();
            emit("open", selected).then();
        }
    }
}