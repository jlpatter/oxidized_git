import {checkUpdate, installUpdate} from "@tauri-apps/api/updater";
import {getVersion} from "@tauri-apps/api/app";
import {relaunch} from "@tauri-apps/api/process";

export class Updater {
    constructor(mainJS) {
        this.mainJS = mainJS;
    }

    checkForUpdates() {
        const self = this;
        checkUpdate().then(async function(updateResult) {
            if (updateResult.shouldUpdate) {
                const updateMessages = updateResult.manifest.body.split(', ');
                updateMessages.forEach((m) => {
                    $('#updateMessages').append($('<li>' + m + '</li>'));
                });
                $('#updateCurrentVersion').text('Current Version: ' + await getVersion());
                $('#updateNewVersion').text('New Version: ' + updateResult.manifest.version);
                $('#updateModal').modal('show');
            }
        }).catch((e) => {
            self.mainJS.showError(e.toString());
        });
    }

    setEvents() {
        const self = this;
        $('#updateBtn').click(async function() {
            const $updaterSpinner = $('#updaterSpinner');
            $updaterSpinner.show();
            try {
                await installUpdate();
                await relaunch();
            } catch (e) {
                self.mainJS.showError(e.toString());
            }
            $updaterSpinner.hide();
            $('#updateModal').modal('hide');
        });
    }
}