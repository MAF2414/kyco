% eigenvalues_plot.m
% Compute eigenvalues of a random 5x5 matrix and plot them.
% Compatible with MATLAB and GNU Octave.
%
% Run (GNU Octave):
%   octave -qf eigenvalues_plot.m
%
% Output:
%   eigenvalues.png

% Reproducible randomness (best-effort across MATLAB/Octave versions).
seed = 0;
if exist('rng', 'file') == 2
    rng(seed);
else
    rand('seed', seed);
    randn('seed', seed);
end

% Random 5x5 matrix and its eigenvalues.
A = randn(5, 5);
lambda = eig(A);

disp('Random 5x5 matrix A:');
disp(A);
disp('Eigenvalues of A:');
disp(lambda);

% Plot eigenvalues in the complex plane and save to disk.
out_file = 'eigenvalues.png';
show_figure = false;
if exist('have_window_system', 'file') == 2 || exist('have_window_system', 'builtin') == 5
    show_figure = have_window_system() ~= 0;
elseif exist('usejava', 'builtin') == 5
    show_figure = usejava('desktop');
end

if show_figure
    fig = figure();
else
    fig = figure('Visible', 'off');
end
plot(real(lambda), imag(lambda), 'o', 'MarkerSize', 8, 'LineWidth', 1.5);
xlabel('Real(\lambda)');
ylabel('Imag(\lambda)');
title('Eigenvalues of a Random 5x5 Matrix');
grid on;
axis equal;
saved = false;
try
    print(fig, out_file, '-dpng');
    saved = true;
catch
end
if ~saved
    try
        print(out_file, '-dpng');
        saved = true;
    catch
    end
end
if ~saved
    print('-dpng', out_file);
end
if ~show_figure
    close(fig);
end

fprintf('Saved eigenvalue plot to %s\n', out_file);
