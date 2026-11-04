import React, { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { request as invoke } from '../../utils/request';
import { showToast } from '../common/ToastContainer';

interface HttpApiSettings {
    enabled: boolean;
    port: number;
}

const DEFAULT_PORT = 19527;
const MIN_PORT = 1024;
const MAX_PORT = 65535;

/**
 * 本机账号切换 API（127.0.0.1，无凭证）的开关与端口。
 * 默认关闭；设置存在 local_account_api.json，下次启动应用时才生效。
 */
const LocalAccountApi: React.FC = () => {
    const { t } = useTranslation();
    const [saved, setSaved] = useState<HttpApiSettings | null>(null);
    const [enabled, setEnabled] = useState(false);
    const [portText, setPortText] = useState(String(DEFAULT_PORT));
    const [saving, setSaving] = useState(false);
    const [restartNeeded, setRestartNeeded] = useState(false);

    useEffect(() => {
        let cancelled = false;
        invoke<HttpApiSettings>('get_http_api_settings')
            .then((settings) => {
                if (cancelled) return;
                setSaved(settings);
                setEnabled(settings.enabled);
                setPortText(String(settings.port));
            })
            .catch((error) => {
                if (!cancelled) showToast(`${t('common.error')}: ${error}`, 'error');
            });
        return () => {
            cancelled = true;
        };
    }, []);

    const port = Number(portText);
    const portValid = Number.isInteger(port) && port >= MIN_PORT && port <= MAX_PORT;
    const dirty = saved !== null && (saved.enabled !== enabled || saved.port !== port);

    const handleSave = async () => {
        if (!portValid) {
            showToast(t('settings.advanced.http_api_port_invalid'), 'error');
            return;
        }
        setSaving(true);
        try {
            await invoke('save_http_api_settings', { settings: { enabled, port } });
            setSaved({ enabled, port });
            setRestartNeeded(true);
            showToast(t('settings.advanced.http_api_settings_saved'), 'success');
        } catch (error) {
            showToast(`${t('common.error')}: ${error}`, 'error');
        } finally {
            setSaving(false);
        }
    };

    return (
        <div className="space-y-3">
            <div className="flex items-center justify-between p-4 bg-gray-50 dark:bg-base-200 rounded-lg border border-gray-100 dark:border-base-300">
                <div>
                    <div className="font-medium text-gray-900 dark:text-base-content">
                        {t('settings.advanced.http_api_title')}
                    </div>
                    <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
                        {t('settings.advanced.http_api_enabled_desc')}
                    </p>
                </div>
                <label className="relative inline-flex items-center cursor-pointer">
                    <input
                        type="checkbox"
                        className="sr-only peer"
                        checked={enabled}
                        disabled={saved === null}
                        onChange={(e: React.ChangeEvent<HTMLInputElement>) => setEnabled(e.target.checked)}
                    />
                    <div className="w-11 h-6 bg-gray-200 dark:bg-base-300 peer-focus:outline-hidden peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-500"></div>
                </label>
            </div>

            <div className="bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-700/30 rounded-lg p-3">
                <p className="text-sm text-amber-700 dark:text-amber-400">
                    {t('settings.advanced.http_api_warning')}
                </p>
            </div>

            <div>
                <label className="block text-sm font-medium text-gray-900 dark:text-base-content mb-1">
                    {t('settings.advanced.http_api_port')}
                </label>
                <div className="flex gap-2">
                    <input
                        type="number"
                        min={MIN_PORT}
                        max={MAX_PORT}
                        className="w-40 px-4 py-3 border border-gray-200 dark:border-base-300 rounded-lg bg-gray-50 dark:bg-base-200 text-gray-900 dark:text-base-content font-medium"
                        value={portText}
                        placeholder={t('settings.advanced.http_api_port_placeholder')}
                        onChange={(e: React.ChangeEvent<HTMLInputElement>) => setPortText(e.target.value)}
                    />
                    <button
                        className="px-4 py-2 border border-gray-200 dark:border-base-300 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-50 dark:hover:bg-base-200 hover:text-gray-900 dark:hover:text-base-content transition-colors disabled:opacity-50"
                        disabled={saving || saved === null || !dirty}
                        onClick={handleSave}
                    >
                        {t('common.save')}
                    </button>
                </div>
                {!portValid && (
                    <p className="text-xs text-red-600 dark:text-red-400 mt-2">
                        {t('settings.advanced.http_api_port_invalid')}
                    </p>
                )}
                <p className="text-xs text-gray-500 dark:text-gray-400 mt-2">
                    {t('settings.advanced.http_api_port_desc')}
                </p>
                {restartNeeded && (
                    <p className="text-xs text-amber-700 dark:text-amber-400 mt-2">
                        {t('settings.advanced.http_api_restart_required')}
                    </p>
                )}
            </div>
        </div>
    );
};

export default LocalAccountApi;
