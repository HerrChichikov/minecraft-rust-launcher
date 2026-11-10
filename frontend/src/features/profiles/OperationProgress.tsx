import { stageLabels, type Progress } from './installationProtocol';

const mib = 1024 * 1024;
const amount = new Intl.NumberFormat('ru-RU', { maximumFractionDigits: 1 });
const percentage = new Intl.NumberFormat('ru-RU', { style: 'percent' });

export function OperationProgress({
  progress,
  active,
  label,
}: {
  progress: Progress;
  active: boolean;
  label?: string | undefined;
}) {
  const { stage, completedBytes, totalBytes, downloadedBytes, elapsedMs } = progress;
  const transferring = stage === 'downloading' || stage === 'repairing';
  const determinate = totalBytes !== null && totalBytes > 0;
  const speed =
    transferring && elapsedMs >= 250 && downloadedBytes > 0
      ? (downloadedBytes * 1000) / elapsedMs
      : null;
  return (
    <div className="operation-progress">
      <div className="progress-heading">
        <p role="status">{label ?? stageLabels[stage]}</p>
        {active && determinate && (
          <span aria-hidden="true">{percentage.format(completedBytes / totalBytes)}</span>
        )}
      </div>
      {active && (
        <>
          <progress
            aria-label="Подготовка файлов"
            max={determinate ? totalBytes : undefined}
            value={determinate ? completedBytes : undefined}
          />
          {determinate && (
            <p className="field-help progress-metrics">
              <span>
                {amount.format(completedBytes / mib)} / {amount.format(totalBytes / mib)} МиБ
              </span>
              {speed !== null && (
                <span>
                  В среднем{' '}
                  {speed >= mib
                    ? `${amount.format(speed / mib)} МиБ/с`
                    : `${amount.format(speed / 1024)} КиБ/с`}
                </span>
              )}
            </p>
          )}
        </>
      )}
    </div>
  );
}
