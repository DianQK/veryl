module ModuleA ;
    always_comb begin
        // if statement
        if (a) begin
            a  = 1;
            aa = 1;
        end else if (a) begin
            a  = 1;
            aa = 1;
        end else begin
            a  = 1;
            aa = 1;
        end

        // for statement
        for (int unsigned a  = 0; a < 10; a++) begin
            a  = 1;
            aa = 1;
        end
    end
endmodule