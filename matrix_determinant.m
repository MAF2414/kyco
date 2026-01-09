% MATLAB-Skript: Erstelle eine 3x3 Matrix und berechne die Determinante

% 3x3 Matrix erstellen
A = [1 2 3; 4 5 6; 7 8 10];

% Matrix anzeigen
disp('Die 3x3 Matrix A:');
disp(A);

% Determinante berechnen
det_A = det(A);

% Ergebnis ausgeben
fprintf('Die Determinante von A ist: %g\n', det_A);
